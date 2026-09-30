# Fixed income and rates: quoting, quantities and rounding

Research note backing `docs/units.md`. Every claim is marked **VERIFIED**
(with a source number from the Sources section) or **UNVERIFIED** (with the
reason). Researched 2026-09-30.

## 1. Government bonds

| Market | Price | Quantity | Accrued interest / rounding |
|---|---|---|---|
| UK gilts | Per £100 nominal, decimal; formulae use the dirty price per £100 **VERIFIED [1][2]** | Nominal £; "can be traded in units as small as a penny" **VERIFIED [2]** | Rounded "to the nearest penny on the traded nominal amount" **VERIFIED [1]** |
| US Treasuries | Per $100 par; auction prices to 6 decimals; cash market in 32nds and fractions of a 32nd | Par $; minimum $100, multiples of $100 **VERIFIED [5][6]** | Per $1,000 par, 5 decimals, "normal rounding", then scaled **VERIFIED [3]** |
| Bunds | Percent of nominal **VERIFIED [7]** | Nominal €; smallest tradeable unit €0.01 **VERIFIED [7]** | Day count actual/actual (ICMA) **VERIFIED [7]**; rounding rule not found **UNVERIFIED** |
| OATs, BTPs | Percent of nominal **UNVERIFIED** (not checked on AFT/MEF pages) | BTP retail issues: €1,000 minimum and multiples **VERIFIED [9]**; wholesale lots not found | Not found **UNVERIFIED** |
| JGBs | Per ¥100 face **VERIFIED [8]** | Face ¥; unit face value ¥50,000 (¥10,000 retail JGBs, ¥100,000 JGBi) **VERIFIED [8]** | Not found **UNVERIFIED** |

**Gilts in detail.** The DMO's price/yield paper [1] defines "nearest
rounding" to n places as "round the nth decimal place up by one if the
(n+1)th decimal place is five or above, and then truncate": for the
positive values it applies to, that is `HalfAwayFromZero`. Rules
(**VERIFIED [1]**):

- Settlement values from yield-to-price: nearest penny on the trade, "with
  no intermediate rounding".
- Accrued interest: nearest penny on the traded nominal.
- Coupons: nearest 6th decimal per £100 nominal.
- Index-linked (3-month lag): reference RPI and index ratio both rounded to
  the nearest 5th decimal; redemption payment to the nearest 6th decimal
  per £100.
- Exception: published cash flows on 4⅛% IL 2030 are "rounded down to 4
  decimal places" per £100 (truncation).

**US Treasuries in detail** (31 CFR 356, Appendix B [3], **VERIFIED**):

- Bills are auctioned on a discount rate, bid to 3 decimals in 0.005 steps
  [4]; price per $100 = 100 × (1 − d × days / 360), "rounded to six
  decimal places, using normal rounding procedures".
- Notes and bonds are bid on yield to 3 decimals [4]; price per $100 is
  rounded to 6 places.
- Coupon interest uses a "daily interest decimal" table printed to 9
  decimals (e.g. $0.227581522 per $1,000 per day): coupon ÷ 2 ÷ days in the
  half-year, which is not a terminating decimal.
- TIPS: "we truncate calculations with regard to the Ref CPI and the Index
  Ratio … to six decimal places, and round to five decimal places"
  (example: 1.000107803 → 1.000107 → 1.00011).
- Stripped TIPS components: adjusted values rounded to 2 decimals "with no
  intermediate rounding".
- FRN accrued interest: 5 decimals per $1,000, "normal rounding".
- "Normal rounding procedures" isn't defined in the text; the examples
  are consistent with round-half-up **UNVERIFIED (inferred)**.

**Cash-market 32nds.** On BrokerTec Chicago, CME's Treasury CLOB, prices
have "nine-decimal (1/16 of 1/32nd) price precision", sent as decimals.
CME publishes a separate guide to converting these to fractional display
**VERIFIED [10]**. 1/512 = 0.001953125, so every 32nd fraction in use is
an exact decimal. BrokerTec halved the 2-year tick to 1/8 of a 32nd in
2018 **VERIFIED (secondary: NY Fed / ICMA library [11])**.

**When-issued trading** is on yield **VERIFIED only via a secondary source
[12]**.

**Clean vs dirty.** The DMO formulae work in dirty price per £100 and add
accrued interest to settlement **VERIFIED [1]**. That the other markets
quote clean and settle clean plus accrued is **UNVERIFIED** (not stated in
the primary sources read).

## 2. Quantities: nominal, not a number of bonds

In every market checked, the traded quantity is a **nominal (face) amount
in currency**. Denominations only set the minimum and the step:

- FIX `QtyType` 0 is "Units (shares, par, currency)"; 1 is "Contracts"
  **VERIFIED [13]**.
- For MiFIR reporting, MarketAxess Trax says bond quantity is
  "Quantity/Nominal Value". But for bonds that are ETCs or ETNs, value is
  `LastQty × LastPx` per unit **VERIFIED [14]**: some exchange-traded bond
  types count units.
- FINRA TRACE: quantity "represents the par value volume of the
  transaction", to 2 decimals **VERIFIED [15]**.
- Börse Frankfurt quotes bonds in percent of nominal (*Prozentnotiz*), not
  per piece **VERIFIED (secondary summary of [16])**.
- Convertibles and retail markets that count bonds by the piece: not
  confirmed **UNVERIFIED**.

The steps are fine-grained: £0.01 (gilts), €0.01 (Bunds), $100 (US
Treasuries), ¥50,000 (JGBs). Because the quantity is currency, a bond
quantity needs at most 2 decimals, and none for JPY.

## 3. Credit and structured

- **Corporate bonds.** FIX `PriceType` has Percentage (1, "percent of
  par"), Spread (6, "basis points spread"), Yield (9), Discount (4), and
  more **VERIFIED [13]**. Trax requires a benchmark (curve and point, or a
  benchmark security) when `PriceType` = 6 **VERIFIED [14]**. TRACE prices
  are percent of par, up to 6 decimals (format 9999.999999) **VERIFIED
  [15]**. Tradeweb's RFQ returning Z/I/G-spread, price and yield:
  **UNVERIFIED** (page returned 403; search snippet only).
- **MBS pool factors.** Fannie Mae: "The factor reflected on the user
  interface is rounded to 8 decimals". Factored UPB (issue UPB × factor)
  can differ from the actual current UPB "due to rounding" **VERIFIED
  [17]**. FIX tag 228 `Factor`: "Qty * Factor * Price = Gross Trade
  Amount". For TIPS the same tag carries the inflation index **VERIFIED
  [13]**. 8 decimals for Freddie Mac, Ginnie Mae and non-agency deals:
  **UNVERIFIED**.
- **Inflation-linked index ratios**: 5 decimals everywhere checked:
  - Gilts: nearest [1].
  - US TIPS: truncate to 6, then round to 5 [3].
  - JGBi: reference index to 3 decimals, indexation coefficient to 5
    [18].
  - All **VERIFIED**. The rounding direction for JGBi isn't stated.
    OATi/€i and Bund linkers: **UNVERIFIED** (AFT page 403; Finanzagentur
    page silent).

## 4. Rates derivatives

- **Swaps and FRAs.** The 2006 ISDA Definitions §8.1: percentages are
  rounded "to the nearest one hundred-thousandth of a percentage point"
  (9.876545% → 9.87655%, i.e. 7 decimals as a fraction). Currency amounts
  go to 2 decimals "with .005 being rounded upwards". §8.2 exceptions: JPY
  and KRW "round down to the next lower whole" unit; CLP and HUF to the
  nearest whole unit, half up **VERIFIED [19]**. The 2021 Definitions'
  rules: **UNVERIFIED** (not publicly available). FIX has `PriceType` 24
  InterestRate and 25 PercentageNotional **VERIFIED [13]**. Swap quantity
  = notional is standard; Trax reports interest rate derivatives' size as
  "Cash Value/Notional" **VERIFIED [14]**.
- **Bond futures.** CME Treasury conversion factors are "rounded to four
  decimal places"; the coupon used is rounded to the nearest 1/8 %
  **VERIFIED via search excerpt of [20]** (direct fetch timed out). Eurex
  factors appear with 6 decimals in examples (e.g. 0.882668) **UNVERIFIED
  (brochure examples only [21])**. CME 2-year note: $200,000 face, tick
  1/8 of 1/32 ($7.8125) **VERIFIED via search excerpt of CBOT rulebook
  ch. 21 [22]**.
- **SOFR futures**: price = 100 − rate; ticks 0.0025 (front) and 0.005 **VERIFIED via
  search excerpt of CME rule 460 [23]**.
- **Repo.** Rates are quoted on the money-market basis, "almost always"
  A/365F or A/360. Haircut = (market value − purchase price) / market
  value × 100 **VERIFIED [24]**. No rounding rule found **UNVERIFIED**.

## 5. Venues

- BrokerTec Chicago: decimal prices with 9 decimals; one lot = $100,000
  (5–30y) or $200,000 (2y, 3y) notional **VERIFIED [10]**. Here the venue
  quantity is **lots**, not nominal: a quantity-unit difference from
  classic BrokerTec to watch for.
- MarketAxess (Trax FIX): `PriceType` 1/2/4/5/6/9/22 supported;
  `LastPx` is "expressed in PriceType (423) units" **VERIFIED [14]**. FIX:
  for fixed income, `AvgPx` is always percent of par, whatever the
  `PriceType` of `LastPx` **VERIFIED [13]**.
- FIX also has `PriceType` 13–19, "product ticks in halves … one-twenty-
  eighths" (including 17, thirty-seconds) **VERIFIED [13]**. So a FIX price
  may be a *count of ticks*, not a decimal: a wire convention to decode at
  the edge.
- Tradeweb, Bloomberg, MTS, eSpeed/Fenics API price formats: **UNVERIFIED**
  (no public specifications found).

## 6. Stress points for Dec19

**Precision.** No convention found needs more than 9 decimals. The finest
were:

| Convention | Decimals |
|---|---|
| Treasury daily interest decimal | 9 |
| BrokerTec price | 9 |
| Pool factors | 8 |
| ISDA rates, as a fraction | 7 |
| Price per 100 (Treasury) | 6 |
| Coupon per £100 (gilts) | 6 |
| Index ratios | 5 |
| CME conversion factors | 4 |

19 places is ample for *stored* values. The pressure is on intermediates:

- **Values that aren't terminating decimals.** Day-count fractions (days/360,
  days/365, days/(2 × 181)), the daily interest decimal, CPI interpolation
  (day − 1)/days-in-month, T-bill price from discount, repo interest, and
  yield ↔ price (powers and roots). 32nds and all binary fractions are
  exact.
- **"No intermediate rounding".** DMO [1] and Treasury [3] both require it
  before the final round to the penny or cent. With a separate multiply,
  then a divide, each rounding to 10⁻¹⁹, the error is about notional ×
  10⁻¹⁹. That is far below a penny for any real trade, but it isn't
  zero. It could flip an exact-half case at the final rounding in
  principle.

**Magnitude.** Notionals are small against ±1.7 × 10¹⁹:

- A ¥1 trillion trade is 10¹².
- The pinch point is aggregates in weak currencies. Global OTC rates
  notional (hundreds of trillions of USD **UNVERIFIED**) expressed in
  IDR or VND would approach 10¹⁹.
- Per-trade intermediates such as nominal × price per 100 (before ÷ 100)
  stay below 10¹⁵ for real trades.

**Rounding modes required** (all present in decimix):

| Rule | Mode |
|---|---|
| DMO "nearest rounding"; ISDA ".005 rounded upwards" | `HalfAwayFromZero` for positive values |
| ISDA JPY/KRW "round down"; 4⅛% IL 2030 "rounded down" | `Floor` / `TowardZero` (same for positives) |
| TIPS index ratio: truncate at 6, then round at 5 | `TowardZero` then `HalfAwayFromZero`; same as one `HalfAwayFromZero` at 5, but not with `HalfEven` |

**Negative values.** Negative rates (EUR, JPY, CHF) make ISDA's "rounded
upwards" ambiguous. It could mean toward +∞ (`Ceiling` on the half) or
away from zero. **UNVERIFIED**: the 2006 text only gives positive
examples.

## Implications for decimix / docs/units.md

- **Glossary holds.** "Fixed income: one asset unit is one unit of the
  face-value currency; quantity is the face amount" is right for gilts,
  Treasuries, Bunds, JGBs, BTPs, TRACE and MiFIR reporting. Drop "(to
  verify)", with two caveats:
  - Some *venues* count **lots** of fixed notional (BrokerTec Chicago:
    $100k/$200k).
  - Some exchange-traded bond-like products (ETCs/ETNs) count **units**.
  - In both cases the venue quantity is stored as quoted, which fits the
    note's principle.
- **Add to the glossary:**
  - *Price per 100 nominal* (percent of par; FIX `PriceType` = 1).
  - *Factor* (pool factor or inflation index ratio). It sits between
    quantity and money (FIX: Qty × Factor × Price), so it is instrument
    data like the contract multiplier.
  - *Clean vs dirty price* and *accrued interest*: a money amount computed
    from the quantity, rounded once to the currency's minor unit.
- **Price conventions beyond decimals:** yield, spread in bp, discount rate
  and FIX tick-count prices (`PriceType` 13–19) all confirm the note's
  question 3. A stored FI price needs its `PriceType`.
- **Precision and range:** Dec19 is sufficient for every stored value
  found. There's no case for more than 19 decimals or more than 1.7 × 10¹⁹
  per trade. Aggregates in very weak currencies are the only range risk.
- **Rounding:** the six modes cover every rule found. Two recommendations
  for the maintainer to decide:
  1. **A fused multiply-then-divide with one rounding** (e.g.
     `mul_div(a, b, c, Round)`) for day-count and accrued-interest
     formulas, so the "no intermediate rounding" rules can be met exactly
     rather than to 10⁻¹⁹. decimix today has `mul_dec` and `div` but no
     fused form.
  2. **Document that `HalfAwayFromZero` is the reading of "nearest"/"round
     half up" in DMO, Treasury and ISDA texts**, with the negative-rate
     ambiguity noted as unresolved.

## Sources

1. UK DMO, *Formulae for Calculating Gilt Prices from Yields*, 4th ed.
   (Dec 2024): <https://www.dmo.gov.uk/media/334d05fo/yldeqns_v4.pdf>
2. UK DMO, *About gilts*: <https://www.dmo.gov.uk/responsibilities/gilt-market/about-gilts/>
3. 31 CFR Part 356, Appendix B: <https://www.law.cornell.edu/cfr/text/31/appendix-B_to_part_356>
4. 31 CFR 356.12 (bid formats): <https://www.law.cornell.edu/cfr/text/31/356.12>; full part: <https://www.treasurydirect.gov/files/laws-and-regulations/auction-regulations-uoc/31-cfr-part-356.pdf>
5. TreasuryDirect, marketable securities FAQ: <https://treasurydirect.gov/help-center/marketable-faqs/>
6. Treasury, "New $100 Minimums" (2008): <https://treasuryauctions.gov/news/2008/release-03-21/>
7. Finanzagentur factsheet DE0001102572: <https://www.deutsche-finanzagentur.de/en/federal-securities/factsheet/isin/DE0001102572>; Federal bonds: <https://www.deutsche-finanzagentur.de/en/federal-securities/types-of-federal-securities/federal-bonds>
8. Japan MOF, *About JGBs*: <https://www.mof.go.jp/english/policy/jgbs/debt_management/guide.htm>; auction result 2026-09-08: <https://www.mof.go.jp/english/policy/jgbs/auction/calendar/eresul/eresul20260908.htm>
9. MEF, BTP Italia FAQ: <https://www.dt.mef.gov.it/it/debito_pubblico/emissioni_retail/btp_italia/btp_italia_faq/>
10. CME, BrokerTec Chicago – U.S. Treasury Actives CLOB: <https://cmegroupclientsite.atlassian.net/wiki/spaces/EPICSANDBOX/pages/649592833/BrokerTec+Chicago+-+U.S.+Treasury+Actives+-+CLOB>
11. *Tick Size Change and Market Quality in the U.S. Treasury Market* (ICMA library): <https://www.icmagroup.org/assets/documents/Regulatory/Secondary-markets/Bond-Market-Liquidity-Library/Tick-Size-Change-and-Market-Quality-in-UST-MarketFed-April-2019-010519.pdf>
12. UCLA, *The Treasury Bill Auction and the When-Issued Market*: <https://www.anderson.ucla.edu/documents/areas/fac/dotm/bio/pdf_SB09.pdf>
13. FIX Trading Community, FIX Latest (EP312) Orchestra repository: `PriceType` (423), `QtyType` (854), `Factor` (228), `AvgPx`. <https://www.fixtrading.org/online-specification/>
14. MarketAxess, *Trax APA FIX Gateway Technical Specification* v1.04: <https://www.marketaxess.com/pdf/APA-FIX-Gateway-Technical-Specification.pdf>
15. FINRA TRACE FAQ and user guide: <https://www.finra.org/filing-reporting/trace/faq>, <https://www.finra.org/sites/default/files/AppSupportDoc/p014513.pdf>
16. Deutsche Börse, *So handeln Sie Anleihen*: <https://www.cashmarket.deutsche-boerse.com/resource/blob/158268/9f3c7ed288fd3a140b2f670084d4d13c/data/Brosch-re-So-handeln-Sie-Anleihen.pdf>
17. Fannie Mae, *Single-Family MBS Disclosures FAQs* (Mar 2023), Q71: <https://capitalmarkets.fanniemae.com/sites/g/files/koqyhd216/files/2023-03/single-family-mbs-disclosures-faqs.pdf>
18. Japan MOF, *Ref Index and Indexation Coefficient*: <https://www.mof.go.jp/english/policy/jgbs/topics/bond/10year_inflation/coefficient.htm>
19. ISDA, *2006 ISDA Definitions*, §8.1–8.2 (copy hosted by Standard Chartered): <https://www.sc.com/en/uploads/sites/66/content/docs/2006-ISDA-Definitions.pdf>
20. CME, *Calculating U.S. Treasury Futures Conversion Factors*: <https://www.cmegroup.com/trading/interest-rates/files/Calculating_U.S.Treasury_Futures_Conversion_Factors.pdf>
21. Eurex, *Fixed Income Trading Strategies*: <https://www.eurex.com/resource/blob/298900/fb9a739a9da7ea294164c90678435c13/data/brochure_fixed_income_trading_strategies_en.pdf>
22. CBOT Rulebook ch. 21 (2-Year T-Note futures): <https://www.cmegroup.com/rulebook/CBOT/II/21.pdf>
23. CME Rulebook ch. 460 (Three-Month SOFR futures): <https://www.cmegroup.com/rulebook/CME/IV/400/460.pdf>
24. ICMA ERCC, *Guide to Best Practice in the European Repo Market* (Nov 2023): <https://www.icmagroup.org/assets/ERCC-Guide-to-Best-Practice-November-2023-FINAL-2-Nov.pdf>
