# Compact decimal vectors: design proposal

Status: **draft for review** (second round; the first round's questions are
answered under *Decisions*). Nothing here is implemented yet.

This answers the design brief's open question about a compact `Fixed64` type
for dense columns. The short version: **yes, as an encoding of a vector of
decimals, not as a second value type.** `Dec19` stays the one value type, on
the wire and in all arithmetic. A vector of many values can be stored as
64-bit integers plus **one scale, stored with them**, and every value converts
to and from `Dec19` exactly or not at all.

## Why

A `Dec19` is 16 bytes, and CPUs have no 128-bit vector lanes. For code that
touches one value at a time (order handling, FIX) that doesn't matter. For
code that scans millions of values (market data, order books, history,
analytics) it does. The evaluation measured, per element on Apple Silicon:

| Operation over a column              | i64     | i128 (`Dec19`) |
|--------------------------------------|---------|----------------|
| Filter / compare with a threshold    | 0.08 ns | 0.39 ns        |
| Sum (overflow-safe, into i128)       | 0.28 ns | 0.28 ns        |
| Memory per value                     | 8 bytes | 16 bytes       |

Filtering and memory bandwidth are where the win is: about 5× faster scans
and half the memory, cache and disk. Sums gain little, because an
overflow-safe sum of i64 values must be accumulated in i128 anyway.

## Decisions (from review)

1. **The scale is a property of the data, stored with it.** A vector is one
   thing: `{ scale, values }`, in memory and on the wire. The scale is never
   held somewhere else (an instrument, a schema, a config), because if it
   could change independently, the stored values would silently change
   meaning. It is typically *chosen* from the instrument's tick precision or
   what the venue provides, but after that it belongs to the data.
2. **Runtime scale**, not a const generic: data with different scales must be
   representable, and a vector's scale can be changed (by rewriting it).
3. **Signed and unsigned**: `DecVec` holds `Dec19` values as i64;
   `UDecVec` holds `UDec19` values as u64. An order book in struct-of-arrays
   form is then a `DecVec` of prices and a `UDecVec` of quantities.
4. **A struct, not a slice plus a scale** in the API:
   `DecVec<V: AsRef<[i64]>>`, generic over its storage so the same type
   covers owned (`Vec<i64>`), borrowed (`&[i64]`) and memory-mapped or
   message-backed data.
5. **Building fails as a whole on `OutOfRange`.** No per-value "missing"
   markers.

## What it is not

- **Not a value type with arithmetic.** There is no element-wise `+` or
  `*` between vectors. You read values out as `Dec19` (exactly) to compute
  with them, so there is one set of arithmetic, rounding rules and tests.
- **Not a per-value scale.** One scale per vector: within a vector every
  value has one representation, so the brief's objections to per-value
  scales (cohorts, alignment on every operation, non-canonical bytes) don't
  apply.
- **Not a bet on a scale that can never change.** A value that doesn't fit
  a vector's scale is rejected, not rounded (unless the caller asks for
  rounding, naming the mode). The caller then keeps a `Vec<Dec19>` or
  rescales.

## Design

### The scale

A `DecVec` stores each `value × 10^scale` as an `i64`, with `scale` from 0
to 18. The scale sets the range: an i64 holds about ±9.2 × 10¹⁸ units, so

| scale | smallest step | largest magnitude |
|-------|---------------|-------------------|
| 2     | 0.01          | about 9.2 × 10¹⁶  |
| 4     | 0.0001        | about 9.2 × 10¹⁴  |
| 8     | 0.00000001    | about 9.2 × 10¹⁰  |
| 12    | 10⁻¹²         | about 9.2 × 10⁶   |
| 18    | 10⁻¹⁸         | about 9.22        |

A `UDecVec` uses u64, doubling the positive range (about 1.8 × 10¹⁹ units).

Tiny-price tokens traded in huge quantities (the brief's warning) may not fit
any scale: then that data stays `Vec<Dec19>`/`Vec<UDec19>`.

### Types

```rust
/// A number of decimal places for a vector: 0 to 18. Checked once, when
/// created.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Scale(u8);

/// Dec19 values stored compactly: each value times 10^scale, as an i64.
/// The scale travels with the values; it is part of the data.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DecVec<V = Vec<i64>> {
  scale: Scale,
  values: V,
}

/// The same for UDec19 values, as u64.
pub struct UDecVec<V = Vec<u64>> { /* scale, values */ }
```

Both fields are private, so a `DecVec` always has a valid scale, and code
can't pair values with the wrong scale by accident.

#### Reading (any `V: AsRef<[i64]>`)

```rust
impl<V: AsRef<[i64]>> DecVec<V> {
  /// Wraps stored data. Any i64 is a valid value at any scale, so this
  /// can't fail: it's how data read from a message or a file is used.
  pub fn from_parts(scale: Scale, values: V) -> Self;

  pub fn scale(&self) -> Scale;
  pub fn len(&self) -> usize;
  pub fn is_empty(&self) -> bool;

  /// One value, widened exactly (can't fail: every i64 at every scale fits
  /// in a Dec19).
  pub fn get(&self, i: usize) -> Option<Dec19>;
  pub fn iter(&self) -> impl Iterator<Item = Dec19> + '_;

  /// The stored integers, for storage, compression or custom scans.
  pub fn raw(&self) -> &[i64];
  pub fn into_parts(self) -> (Scale, V);

  /// A borrowed view of the same data.
  pub fn as_ref(&self) -> DecVec<&[i64]>;
}
```

#### Building (`V = Vec<i64>`)

```rust
impl DecVec<Vec<i64>> {
  pub fn new(scale: Scale) -> Self;
  pub fn with_capacity(scale: Scale, n: usize) -> Self;

  /// Exact: fails if any value has more decimal places than the scale
  /// (`Inexact`) or doesn't fit an i64 at this scale (`OutOfRange`), and
  /// says which value. Nothing is built on failure.
  pub fn from_decs(scale: Scale, xs: &[Dec19]) -> Result<Self, BuildError>;

  /// Rounds each value to the scale with `mode`; fails only on
  /// `OutOfRange` (for the whole batch).
  pub fn from_decs_round(scale: Scale, xs: &[Dec19], mode: Round)
    -> Result<Self, BuildError>;

  /// Appends one value, exactly; on error the vector is unchanged.
  pub fn push(&mut self, x: Dec19) -> Result<(), BuildError>;

  /// The same values at another scale: exact or an error (a finer scale can
  /// overflow; a coarser one can be inexact).
  pub fn rescale(&self, scale: Scale) -> Result<Self, BuildError>;
}

/// Why a value couldn't be stored, and which one.
pub struct BuildError { pub index: usize, pub kind: BuildErrorKind }
pub enum BuildErrorKind { Inexact, OutOfRange }
```

`UDecVec` mirrors all of this with u64 and `UDec19`.

### Operations on the compact form

These run at i64 speed and are the reason the type exists:

- `sum(&self)` accumulates in i128 (exact; can't overflow for any realistic
  length), then widens once. The widened sum can exceed `Dec19`'s range in
  theory, so it returns `Result<Dec19, OutOfRange>`.
- `min(&self)`, `max(&self) -> Option<Dec19>`.
- **Comparisons with a `Dec19` threshold**, e.g. "prices above 113.725":
  the threshold is converted to the vector's scale once, then integers are
  compared. The threshold may have more decimal places than the vector, so
  the conversion rounds in the direction that keeps each comparison exact
  (t in vector units, x a stored integer):
  - `x > t` exactly when `x > floor(t)`,
  - `x >= t` exactly when `x >= ceil(t)`,
  - `x < t` exactly when `x < ceil(t)`,
  - `x <= t` exactly when `x <= floor(t)`,
  - `x == t` is false for every x if t isn't a whole number of units.

  A threshold beyond the i64 range at this scale makes a comparison all-true
  or all-false; the functions handle that rather than failing. The API
  shape (a count, positions, a bitmap) is an open question below.

### Searching, sorting and slicing

Within one vector, integer order is value order (every value is the same
power of ten times its integer), so sorting and searching work directly on
the stored integers.

```rust
impl<V: AsRef<[i64]>> DecVec<V> {
  /// A sub-range as a borrowed vector with the same scale, e.g.
  /// `book.slice(0..book.lower_bound(px))` for every level below `px`.
  /// Panics if the range is out of bounds, like slice indexing;
  /// `get_slice` returns `None` instead.
  pub fn slice(&self, range: impl RangeBounds<usize>) -> DecVec<&[i64]>;
  pub fn get_slice(&self, range: impl RangeBounds<usize>)
    -> Option<DecVec<&[i64]>>;
  pub fn split_at(&self, mid: usize) -> (DecVec<&[i64]>, DecVec<&[i64]>);

  /// In a sorted vector: the first position whose value is >= `x`
  /// (`lower_bound`) or > `x` (`upper_bound`), as in C++. `x` may have more
  /// decimal places than the vector; the search uses the exact comparison
  /// rules above (`>= x` is `>= ceil(x)` in vector units, `> x` is
  /// `> floor(x)`), so the answer is exact either way.
  pub fn lower_bound(&self, x: Dec19) -> usize;
  pub fn upper_bound(&self, x: Dec19) -> usize;

  /// Like `slice::binary_search`: `Ok(i)` if `x` is stored at `i`, or
  /// `Err(i)` with where it would go. A value with more decimal places than
  /// the scale is never stored, so that gives `Err(lower_bound(x))`.
  pub fn binary_search(&self, x: Dec19) -> Result<usize, usize>;

  pub fn is_sorted(&self) -> bool;

  /// The positions that would sort the vector (stable), for reordering
  /// several vectors of a struct-of-arrays together: sort the prices'
  /// positions, then `permute` the prices and the quantities with them.
  pub fn sorted_indices(&self) -> Vec<usize>;
}

impl<V: AsMut<[i64]> + AsRef<[i64]>> DecVec<V> {
  /// Sorts in place (sort_unstable on the integers).
  pub fn sort(&mut self);
}

impl DecVec<Vec<i64>> {
  /// A new vector with `self[indices[k]]` at position k.
  pub fn permute(&self, indices: &[usize]) -> Self;
}
```

A slice keeps the scale because it borrows from the same data; there's no
way to get the integers of a slice without it except `raw()`, deliberately.

These are plain loops the compiler can vectorise. Explicit SIMD is out of
scope for a first version (portable code first, `core::arch` only if a
benchmark justifies it).

### Storage

- **Cap'n Proto**: the vector is one struct, so the scale can't be
  separated from the values:

  ```capnp
  struct DecVec  { scale @0 :UInt8; values @1 :List(Int64);  }
  struct UDecVec { scale @0 :UInt8; values @1 :List(UInt64); }
  ```

  Individual values in messages stay `Dec19`. Whether a message-backed
  list can be used directly as `V` (without copying) depends on the capnp
  reader handing out a `&[i64]`; see the questions.
- **Order-preserving keys**: a stored value can use the standard i64 key
  encoding (big-endian, sign bit flipped), which sorts the same as the
  widened values *within one vector*. Keys from vectors with different
  scales, or from `Dec19` keys, don't compare with each other; use
  `Dec19::to_key_bytes` where they mix.
- **Changing scale** is `rescale`, which rewrites the values.

### Performance targets

- `get`/`iter`: one multiply by a per-scale constant, about 1 ns per value.
- `from_decs`/`push`: `to_scaled`-like, currently about 4.9 ns per value;
  the bulk version hoists the per-scale constant out of the loop.
- Filters and min/max at i64 speed (about 0.1 ns per element); sum about
  0.3 ns.

## Verification

- For every scale 0 to 18: `from_decs` succeeds exactly when the
  big-integer oracle says every value fits exactly, reports the first
  failing index and kind otherwise, and `iter` gives the inputs back.
  `from_decs_round` against the oracle in all six rounding modes.
- The threshold comparisons against a direct `Dec19` comparison of every
  element, for thresholds with more, the same and fewer decimal places than
  the vector, and thresholds outside its range.
- `sum`, `min`, `max` against the same operations on widened values.
- `rescale` round trips, and its failures against the oracle.
- `lower_bound`, `upper_bound` and `binary_search` against a linear scan of
  widened values, for thresholds with more, the same and fewer decimal
  places than the vector, below the first and above the last element, and
  with duplicates. `sort` and `sorted_indices` against sorting the widened
  values.
- Benchmarks against `Vec<Dec19>` for the same operations.

## Questions for review

1. **Names**: `DecVec`/`UDecVec` (proposed), and `BuildError` for why a
   value couldn't be stored. Does `from_decs` read well, or
   `try_from_decs` / `from_slice`?
2. **Inexact values when building**: `from_decs` rejects values with more
   decimal places than the scale (`Inexact`), and `from_decs_round` rounds
   them with a named mode. Is that the right pair, or should building
   always require the caller to pick one?
3. **Threshold filters on unsorted data**: for sorted data, `lower_bound`
   plus `slice` covers it. For unsorted data, which shape is useful: a
   count, the matching positions, a bitmap, or an iterator? Start with one.
4. **Reordering several vectors together**: `sorted_indices` plus
   `permute` (proposed), or a helper that sorts one vector and applies the
   same order to others in one call?
5. **Cap'n Proto without copying**: as far as I know (not yet verified),
   capnp-rust's `primitive_list::Reader` can give a `&[i64]` only when the
   data is suitably aligned and the machine is little-endian. If so, is
   copying into a `Vec<i64>` acceptable when it can't, or should `DecVec`
   accept a small trait of its own instead of `AsRef<[i64]>`?
6. **Where it lives**: in `decimix` (proposed; it's small and depends only
   on the core types), or a separate crate?
