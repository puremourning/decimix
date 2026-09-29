# Compact decimal columns: design proposal

Status: **draft for review**. Nothing here is implemented yet.

This answers the design brief's open question about a compact `Fixed64` type
for dense columns. The short version: **yes, but as a storage format for
columns, not as a second value type.** `Dec19` stays the one value type,
on the wire and in all arithmetic. A column of many values can be stored as
64-bit integers with **one scale for the whole column**, and every value
converts to and from `Dec19` exactly or not at all.

## Why

A `Dec19` is 16 bytes, and CPUs have no 128-bit vector lanes. For code that
touches one value at a time (order handling, FIX) that doesn't matter. For
code that scans millions of values (market data, history, analytics) it does.
The evaluation measured, per element on Apple Silicon:

| Operation over a column | i64 | i128 (`Dec19`) |
|---|---|---|
| Filter / compare with a threshold | 0.08 ns | 0.39 ns |
| Sum (overflow-safe, into i128) | 0.28 ns | 0.28 ns |
| Memory per value | 8 bytes | 16 bytes |

Filtering and memory bandwidth are where the win is: about 5× faster scans
and half the memory, cache and disk. Sums gain little, because an overflow-safe
sum of i64 values must be accumulated in i128 anyway.

## What it is not

- **Not a value type with arithmetic.** There is no `Fixed64 + Fixed64`,
  no multiply. You read values out as `Dec19` (exactly) to compute with
  them. That keeps one set of arithmetic, rounding rules and tests.
- **Not a per-value scale.** The scale belongs to the column (its schema or
  metadata), never to individual values. So there is still one
  representation per value within a column, and the brief's objections to
  per-value scales (cohorts, alignment on every operation, non-canonical
  bytes) don't apply.
- **Not a bet on a scale that can never change.** The brief rejected a
  fixed per-domain scale because the first venue with a finer tick breaks
  it. Here a value that doesn't fit the column's scale is **rejected**, not
  rounded, and the caller keeps a `Dec19` column instead (or rewrites the
  column at a finer scale). Nothing is lost silently.

## Design

### The scale

A column stores `value × 10^scale` as an `i64`, with `scale` from 0 to 18.
The scale sets the range: an i64 holds about ±9.2 × 10¹⁸ units, so

| scale | smallest step | largest magnitude |
|---|---|---|
| 2 | 0.01 | about 9.2 × 10¹⁶ |
| 4 | 0.0001 | about 9.2 × 10¹⁴ |
| 8 | 0.00000001 | about 9.2 × 10¹⁰ |
| 12 | 10⁻¹² | about 9.2 × 10⁶ |
| 18 | 10⁻¹⁸ | about 9.22 |

A natural choice is the instrument's tick size (a tick of 0.005 needs scale
3) or the venue's published precision. Tiny-price tokens traded in huge
quantities (the brief's warning) may not fit any scale: then the column
stays `Dec19`.

### Types

```rust
/// A column scale: 0 to 18 decimal places. Checked once, when created.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Scale(u8);

impl Scale {
  pub const fn new(decimal_places: u8) -> Option<Scale>;
  pub const fn decimal_places(self) -> u8;

  /// Exact: every i64 at every scale fits in a Dec19.
  pub fn widen(self, v: i64) -> Dec19;

  /// Exact or an error: `Inexact` if the value has more decimal places than
  /// the scale, `OutOfRange` if it's too big for an i64 at this scale.
  pub fn narrow(self, x: Dec19) -> Result<i64, NarrowError>;

  /// Rounds to the scale with `mode`; `OutOfRange` if still too big.
  pub fn narrow_round(self, x: Dec19, mode: Round) -> Result<i64, OutOfRange>;

  // Bulk versions, for building and reading columns.
  pub fn widen_into(self, src: &[i64], dst: &mut [Dec19]);
  pub fn narrow_into(self, src: &[Dec19], dst: &mut [i64])
    -> Result<(), (usize, NarrowError)>; // index of the first failure
}
```

`widen` can't fail: `|i64| × 10^(19 − scale)` is at most about 9.2 × 10³⁷,
inside an i128. `narrow` divides by `10^(19 − scale)` and requires a zero
remainder.

Columns themselves are just `&[i64]` plus a `Scale`, so they work directly
on borrowed data (a Cap'n Proto `List(Int64)`, a memory-mapped file, an
Arrow buffer) without copying into an owned type. A small owned wrapper
(`Column64 { scale, values: Vec<i64> }`) is optional convenience.

### Column operations

Operations that are worth doing on the compact form, because they run at i64
speed, are provided on `(Scale, &[i64])`:

- `sum(scale, &[i64]) -> Dec19` accumulates in i128 (exact, can't overflow
  for any realistic length), then widens once.
- `min`, `max` return the widened value.
- **Comparisons with a `Dec19` threshold**, e.g. "prices above 113.725",
  convert the threshold to the column's scale once and then compare
  integers. The threshold may have more decimal places than the column,
  so the conversion must round in the direction that keeps the comparison
  exact:
  - `x > t` exactly when `x > floor(t)` (t in column units),
  - `x >= t` exactly when `x >= ceil(t)`,
  - `x < t` exactly when `x < ceil(t)`,
  - `x <= t` exactly when `x <= floor(t)`,
  - `x == t` is false for every x if t isn't a whole number of column
    units.

  A threshold outside the i64 range at this scale makes the comparison
  all-true or all-false; the functions handle that rather than failing.

These are plain loops written so the compiler can vectorise them. Explicit
SIMD is out of scope for a first version, per the plan's order: portable
code first, `core::arch` only if a benchmark justifies it.

### Performance targets

- `widen`: one multiply by a per-scale constant, about 1 ns.
- `narrow`: `to_scaled` now costs about 4.9 ns (the kernel's reciprocal
  divide-by-10¹⁹ plus one 64-bit division). Because the scale is fixed per
  column, the bulk versions can hoist the per-scale constant out of the
  loop. A const-generic variant (see the questions) would turn the division
  into a multiply at compile time.
- Filters and min/max at i64 speed (about 0.1 ns per element); sum about
  0.3 ns.

### Storage

- **Cap'n Proto**: a column is `List(Int64)` plus a scale field
  (`UInt8`) in the enclosing struct. Individual values in messages stay
  `Dec19`.
- **Order-preserving keys**: an i64 column value can use the standard i64
  key encoding (big-endian, sign bit flipped), which sorts the same as
  the widened `Dec19` values *within one column*. Keys from columns with
  different scales, or from `Dec19` keys, don't compare with each other;
  use `Dec19::to_key_bytes` where keys from different sources mix.
- **Changing a column's scale** means rewriting the column. Going to a
  finer scale is exact but can overflow; going to a coarser one is exact
  only if every value fits (use `narrow`, which reports the first failure).

### Unsigned columns

A `u64` variant for quantities (paired with `UDec19`) doubles the positive
range. It's the same design; whether it's needed is a question below.

## Verification

- `widen(narrow(x)) == x` whenever `narrow` succeeds, and `narrow` fails
  exactly when the big-integer oracle says the value doesn't fit exactly,
  for every scale 0 to 18.
- The threshold comparisons against a direct `Dec19` comparison of every
  element, for thresholds with more, the same and fewer decimal places
  than the column, and thresholds outside the column's range.
- `sum`, `min`, `max` against the same operations on widened values.
- Benchmarks against a `Dec19` column for the same operations.

## Questions for review

1. **Runtime scale or const-generic scale, or both?** A runtime `Scale`
   fits "scale in the schema, chosen per instrument or dataset". A
   `Fixed64<const SCALE: u8>` is faster (division by a compile-time
   constant becomes a multiply) and type-checks scale mismatches, but the
   scale must be known when compiling. Proposal: runtime `Scale` first,
   add a const-generic wrapper only if benchmarks show the division
   matters.
2. **Unsigned (u64) columns**: needed now, or later?
3. **Name**: `Scale` + free functions over `(Scale, &[i64])`, or a
   `Column64` type as the main API? And should this live in `decimix` or
   in a separate crate?
4. **Where the scale comes from**: is it always the instrument's tick
   precision, a per-venue setting, or per dataset? This affects only
   documentation and helpers, not the core design.
5. **Out-of-range policy when building a column**: fail the whole batch
   (proposed), or let the caller choose per value (e.g. mark as missing)?
