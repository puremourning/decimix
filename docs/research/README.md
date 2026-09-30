# Market conventions research

Research into how prices and quantities are expressed across asset classes,
done to check the design of decimix against real markets: its range and
precision, its rounding modes, and the units rules in
[`../units.md`](../units.md). It is broader than decimix and meant to be
useful for the wider platform too.

Each note cites primary sources, marks every claim VERIFIED or UNVERIFIED,
and ends with its implications for decimix. Researched 2026-09-30 by
research agents working from public documentation; worth re-checking before
relying on a detail marked UNVERIFIED.

| Note | Covers |
|---|---|
| [`listed-and-fix.md`](listed-and-fix.md) | FIX `PriceType`/`QtyType` and datatypes; CME price encoding, `DisplayFactor`, Treasury and SOFR futures; ASX Trade and ASX 24; equities |
| [`fixed-income-and-rates.md`](fixed-income-and-rates.md) | Government bonds (gilts, Treasuries, Bunds, JGBs), quantities, accrued interest and its rounding, MBS pool factors, inflation index ratios, swaps, repo, electronic venues |
| [`fx-and-crypto.md`](fx-and-crypto.md) | FX quantities, currency minor units, pips, forwards, settlement rounding; crypto exchange APIs, derivatives, on-chain base units, token decimals and supplies |

Summary of what it found for decimix:

- Nothing stored needs more than 19 decimal places (the most seen was 10
  for exchange values, 9 for fixed-income calculations), or comes near
  ±1.7 × 10¹⁹ per value.
- Every market rounding rule found maps onto one of decimix's six rounding
  modes. The one gap is a fused multiply-then-divide with a single rounding,
  for day-count and interest formulas.
- What doesn't fit `Dec19` is a question of units: whole base units (wei)
  and assets with 24 decimals, both handled at the edges as described in
  `units.md`.
