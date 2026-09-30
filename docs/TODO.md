# TODO

Things decided or noted but not done yet. Done items are removed rather than
ticked; the git log has the history.

## Now / next

- [ ] **Compact decimal columns**: review the proposal in
  [`compact-columns.md`](compact-columns.md), settle its open questions,
  then implement.
- [ ] **Check CI** on the `dec19-core` branch: all-features matrix, the
  `no-features` and `float-lint` jobs, Miri timing, and the first
  coverage-guided fuzz runs (`Fuzz` workflow, runs on `main` or on demand).

## Performance

- [ ] `Display` and the f64 slow path still re-check UTF-8 on text that is
  ASCII by construction (`AsciiBuf::as_str` already skips it).
- [ ] Formatting a short value (~19 ns) is about 2× `itoa` on the raw
  integer; the remaining cost is a chain of dependent multiplies.
- [ ] `from_f64_lossy` spends ~43 ns in std's shortest-decimal formatting;
  a direct shortest-digits algorithm would be faster if it ever matters.
- [ ] General `div` (~30 ns, 256-bit via `ethnum`) is fine for now; a
  hand-written division only if profiling says so.

## Verification

- [ ] Kani proofs: the rounding decision (`round::decide`) and the
  sign/overflow logic.
- [ ] Reduced-width kernel test (brief's verification plan, item 2): run the
  divide-by-constant kernel generically on 8- or 16-bit words over every
  input. So far only the rounding helper is tested exhaustively.

## Design decisions (parked)

- [ ] **Cap'n Proto encoding** of the 128-bit value: `lo`/`hi` words or a
  fixed-width byte array. Same question as for UUIDs; decide together.
- [ ] **Priced / unpriced**: a price is in effect `Option<Price>`. Choose
  the representation (a reserved "no price" value vs `Option`).
- [ ] **Price units**: still to decide.
- [ ] **`PriceOffset`** (FIX datatype): deferred; Price + Price doesn't
  materially differ from Price + PriceOffset.
- [ ] `ProductSum::div` reports a zero denominator as `OutOfRange`; a
  distinct error (or `Option`) would be clearer.

## Features

- [ ] Wide analytics accumulator: `ProductSum` already sums products and
  squares exactly (`acc.add(x, x)`), but can't combine totals, e.g.
  n·Σx² − (Σx)² for a variance. Low priority.
- [ ] Wider quantity support for very large base-unit amounts (e.g. wei):
  these must be scaled at ingest today (`from_scaled`); `Dec19` holds about
  ±1.7 × 10¹⁹ whole units.

## Integrations

- [ ] Cap'n Proto newtype support (via the `newtype!` feature hook,
  `__newtype_features!`).
- [ ] Selecta support.
- [ ] serde (a feature; values as strings by default, floats only by
  explicit opt-in).

## Release

- [ ] Publish `decimix` 0.1.0 (Release workflow).
- [ ] Decide whether `decimix-finance` is published (currently
  `publish = false`) and whether its domain types are worth keeping.
