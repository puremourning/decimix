# decimix — design brief (fixed-point decimal for a trading platform)

This brief summarises a design discussion and the conclusions reached. Treat the
**Decisions** as settled unless told otherwise; **Open questions** are yours to
raise, not to decide silently. A verified reference implementation of the
multiply path, `dec19.rs`, accompanies this brief.

## Context

- Low-latency trading platform in Rust (exchange connectivity/FIX, order handling,
  custom storage, frontend). Hot paths care about nanoseconds and allocation-free code.
- Current state: decimals are `(i64 mantissa, u8 scale)` both in Cap'n Proto
  messages and internally. This is being **replaced entirely**, including in
  messages.
- Most arithmetic in DMA/front-office code is compare, add/sub, price × qty, and
  tick/lot snapping. General decimal division is rare (averages, ratios, analytics).

## Decisions

### Representation
- **`Dec19`: an `i128` mantissa at a fixed scale of 19** (value = mantissa × 10⁻¹⁹).
  Range ±1.7×10¹⁹ whole units, resolution 10⁻¹⁹: all 38-digit values below 10¹⁹,
  and 39 digits at the top of the range (i128::MAX has 39 digits).
- Crate name **`decimix`**, main type **`decimix::Dec19`**. If other decimal types are
  added later (e.g. `Dec9`, `Dec38`), they live under the same crate.
- Messages carry `Dec19` as i128 (e.g. two u64 halves in Cap'n Proto). No per-value
  scale anywhere.
- One representation per value, so derived `Eq`/`Ord`/`Hash` are correct.

### Why this, and not the alternatives (don't re-litigate)
- **Floating decimal (per-value scale, e.g. current `(i64,u8)`, `rust_decimal`,
  IEEE decimal64/128, libdecNumber):** every op aligns exponents; cohorts
  (`1.0` vs `1.00`) break structural equality/hashing; slow. libdecNumber was
  previously removed from an F&O DMA system for a ~10% speedup.
- **Per-domain compile-time scale (`Fixed<const S>` with e.g. price at 1e-9):**
  choosing S is an invented multiplier that can never change; the first venue with a
  finer tick breaks it, and persisted data depends on it.
- **Scale derived per instrument from tick size:** still a bet. A scale trades range
  for precision (i64 at scale 15 tops out at ~9,223); prices are unbounded and can be
  negative; ticks like 10/25/50 imply negative exponents; variable tick tables give
  several scales per instrument.
- **Venue multipliers (CME DisplayFactor, Bloomberg's unpublished factors):** someone
  has to invent them and they can never change. Rule: **fold venue display factors in
  at ingest** (decimal × decimal is exact), so no multiplier leaks inward.
- **Scale 19 in i128 is chosen because it exceeds any real price and any real tick,
  so nobody ever picks a multiplier.**

### Arithmetic (implemented or specified)
- **Add/sub/compare:** plain i128 ops (~1 ns). Exact.
- **Multiply `Dec19 × Dec19`:** 128×128 → 256-bit product, then ÷10¹⁹ with an explicit
  rounding mode. Implementation in `dec19.rs`:
  - four `mulx` partial products assembled into 4 × u64 limbs;
  - overflow if limb3 ≠ 0 or limb2 ≥ 10¹⁹;
  - 10¹⁹ fits one limb and has its top bit set (already normalised), so division is
    **two chained 2-by-1 steps using the Möller–Granlund precomputed reciprocal**
    (`V = floor((2¹²⁸−1)/10¹⁹) − 2⁶⁴`), no hardware `div`;
  - branchless rounding from the final remainder (HalfEven, Floor implemented).
- **Fast path, detected (not assumed):** if one operand is a whole number (e.g.
  integer qty), 10¹⁹ = 2¹⁹·5¹⁹, so: low 19 bits zero **and** `(|b|>>19) × inv(5¹⁹) mod
  2¹²⁸ ≤ u128::MAX/5¹⁹`. That product *is* the exact quotient n (always fits u64);
  then result = a × n via 128×64, exact, no division. See `whole()` / `mul19_fast()`.
- **Sums of products** (notional, VWAP numerators): accumulate at scale 38 in a 256-bit
  accumulator, divide once at the end. Faster and more exact.
- **Division, by case:**
  1. `div_euclid(self, rhs) -> i128` and `rem_euclid(self, rhs) -> Dec19` (named as in
     std; originally `div_floor`, renamed because with a negative divisor Euclidean
     division doesn't round down, which clashed with `Round::Floor`): scales cancel,
     so these are i128 `div_euclid`/`rem_euclid` on mantissas. Exact. Covers lot counts,
     clip counts, **tick snapping** (`px.div_euclid(tick) * tick`).
  2. `div_int(self, n, Round) -> Dec19`: mantissa ÷ integer, remainder drives rounding.
     Use `whole()` to detect integer-valued divisors at runtime.
  3. General `div(self, rhs, Round)`: `(aₘ × 10¹⁹) / bₘ` needs 192-by-128 division with a
     runtime divisor (Knuth Algorithm D; its add-back correction is a rare, bug-prone
     path). **Don't hand-write it initially:** use a 256-bit integer crate (e.g.
     `ethnum`) and round from the remainder. Rare path; tens of ns is acceptable.
- **Negative values are normal** (negative prices, spreads, negative tick-table bands).
  Rust integer `/` truncates toward zero, so **anything that snaps to a grid must use
  `div_euclid` or explicit floor/ceil.**

### API rules ("idiot-proof")
- `pub struct Dec19(i128)` with a private field; `#[must_use]` throughout.
- **No float in, no float out by default:** no `From<f64>`, no `TryFrom<f64>`, no
  `as_f64`. A single `to_f64_lossy()` exists and is banned in core crates via
  `clippy.toml` `disallowed-methods`, allowed with `#[allow]` only in analytics crates.
  (Do **not** gate it with a Cargo feature: feature unification would enable it
  workspace-wide.)
- Literals via `dec!("113.725")`, parsed by a `const fn` so a bad literal is a compile
  error. `FromStr`/`Display` are exact and locale-free.
- **Operators only for exact operations:** `+ - neg`, comparisons, × integer, and
  `%` (truncated remainder, as on Rust integers; exact, since it's smaller than the
  divisor).
  Anything that rounds (decimal × decimal, all division) is a method with a
  **mandatory rounding argument**, e.g. `px.mul(qty, Round::HalfEven)`.
- Overflow: operators panic (also in release); `checked_*` return `Option`. At 1.7×10¹⁹
  whole units, overflow is a bug, not data.
- Rounding modes: at least HalfEven (money), Floor, Ceiling, TowardZero, AwayFromZero.
- Domain newtypes (`Price`, `Qty`, `Notional`, `Rate`) wrap `Dec19` so `price + qty`
  doesn't compile and `Price × Qty → Notional` is the only defined product. Provide
  `Notional::sum_products(iter)` over the 256-bit accumulator.
- Serialise as string in JSON; two u64 halves (or i128) in Cap'n Proto.
- Conversion rule: `From` only where lossless; anything that can round names its
  rounding.
- Slow path for scientific work: `From<Dec19> for bigdecimal::BigDecimal` (always exact:
  `BigDecimal::new(BigInt::from(m), 19)`), and back via
  `Dec19::from_big(x, Round) -> Result<Dec19, OutOfRange>`. Behind an optional Cargo
  feature (safe to unify: additive and exact). decimal128 (34 digits) and `rust_decimal`
  (~28 digits) **cannot** hold all 38 digits; only offer them as named `_lossy`
  conversions if at all.

### Verification plan
1. Differential tests against a big-integer oracle (`proptest` + `num-bigint`),
   generators biased to edges (powers of ten, ±1, exact halves, i128 limits), plus
   `cargo-fuzz` in CI.
2. **Exhaustive testing at reduced width:** write kernels generic over limb type and
   divisor, run the same algorithm on u8/u16 limbs over all inputs. Best evidence for
   carry/correction/rounding logic, especially division.
3. Kani proofs on reduced-width instances, and at full width only on multiplication-free
   logic (sign, overflow, rounding). SAT solvers struggle with 64/128-bit multiplies.
4. Deductive proofs (Verus/Creusot) only if the above ever miss a bug.

## Measured results (shared cloud Xeon 2.8 GHz, rustc 1.91; trust ratios over absolutes)

| Operation | ns/op |
|---|---|
| `mul19` Dec19 × Dec19, reciprocal division | ~14.5 (~40 cycles, latency-bound) |
| same, hardware `div` instead of reciprocal | ~61 |
| `mul19_fast`, integer operand detected | ~6.5 |
| fast-path check when it misses | ~+0.5 |
| bare 128×128 multiply, no division | ~1.9 |
| f64 multiply | ~1.2–1.5 |

`mul19` and `mul19_fast` were verified against a Python big-int reference on 200k random
cases plus edge cases, HalfEven and Floor, release and debug builds. Re-benchmark
hardware `div` on production CPUs; newer cores divide much faster.

## Existing crates (evaluated)

- **`primitive_fixed_point_decimal` 1.5.0**: `ConstScaleFpdec<i128, 19>` is exactly this
  representation, with mixed-scale mul and result scale chosen by type. 0 mismatches on
  the same 200k cases. ~26 ns per mul. **No half-even rounding.** Exposes float
  conversions. Option: wrap it behind the `Dec19` newtype initially, then swap in own
  kernels where profiling says so.
- **`rust_decimal`**: floating, 96-bit mantissa. **`fixed`**: binary, not decimal.
- **`bigdecimal` 0.4.11**: by-value Eq/Hash (like Python), but `TryFrom<f64>` gives the
  exact binary expansion of 0.1, `/` silently rounds to 100 digits, scale grows on mul;
  add ~43 ns, mul ~17 ns (small values), div ~4,650 ns. Fine as the lossless slow-path
  target only.
- Java `BigDecimal` / Python `Decimal` hazards the design removes: float constructors,
  Java `equals` including scale, Java `divide` throwing on non-terminating results,
  Python's silent context rounding (28 digits default), scale growth, allocation per op.

benchmark vs floating point: https://docs.rs/fpdec/latest/fpdec/
and fixed poitn: https://docs.rs/primitive_fixed_point_decimal/latest/primitive_fixed_point_decimal/


## Open questions (raise, don't assume)

- A compact `Fixed64` (i64 at a chosen scale) for dense quantity columns (half the cache
  footprint, SIMD-able)? Widening to Dec19 is always exact. Caution: tiny-price tokens
  trade in huge quantities, and a 64-bit scale is a bet again. Keep i128 on the wire.
- Whether domain newtypes live in `decimix` or a downstream crate.
- Exact Cap'n Proto encoding for i128.
- Whether to depend on `primitive_fixed_point_decimal` at all or ship own kernels from
  day one.
- Documentation must state "19 **decimal places**" prominently: SQL readers may assume
  `DECIMAL(19)` means 19 total digits.

## Rough cost

Core ~1,500–2,500 lines plus tests. Add/sub/compare trivial; multiply done; the time
goes into division (via 256-bit crate first) and exact parse/format. A few weeks for a
verified core, about as long again for newtypes, serialisation and lint rules.
