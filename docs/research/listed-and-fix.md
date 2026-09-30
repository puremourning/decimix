# FIX conventions, equities and listed derivatives

Research backing `docs/units.md`. Each claim is marked **VERIFIED** (with a
source number, see Sources) or **UNVERIFIED** (with the reason). Checked on
2026-09-30.

## 1. FIX

### Datatypes the domain types are named after

From the FIX Latest Orchestra repository, EP312 [1]. All are subtypes of
`float`, which is text: digits, optional sign and decimal point, "must
accommodate up to fifteen significant digits", with the number of decimal
places left to "business/market needs and mutual agreement". VERIFIED [1].

| Datatype | Definition (abridged from [1]) |
|---|---|
| `Price` | "float field representing a price". Decimal places may vary; may be negative for some asset classes (e.g. option strategies). |
| `Qty` | Whole number of shares, or a decimal for "securities denominated in fractional units". |
| `Amt` | "float field typically representing a Price times a Qty". |
| `Percentage` | A **fraction**: "0.05 represents 5% and 0.9525 represents 95.25%". |
| `PriceOffset` | "can be mathematically added to a Price"; e.g. `LastForwardPoints` (195): 61.99 points is sent as 0.006199. |

VERIFIED [1]. Note that FIX types FX rates as `Price` (`LastSpotRate` 194)
or plain `float` (`SettlCurrFxRate` 155), not `Percentage`. The
`decimix-finance` doc comment for `Percentage` mentions FX rates; that is
not FIX's usage (see Implications).

### `PriceType` (423)

"Code to represent the price type." For financing trades it gives the repo
type. VERIFIED [1].

| Value | Name | Meaning |
|---|---|---|
| 1 | Percentage | Percent of par ("dollar price" for fixed income) |
| 2 | PerUnit | Per share or contract |
| 3 | FixedAmount | Absolute amount |
| 4 / 5 | Discount / Premium | Percentage points below / over par |
| 6 | Spread | Basis-point spread (e.g. to a benchmark) |
| 7 / 8 | TEDPrice / TEDYield | |
| 9 | Yield | |
| 10 / 11 | Fixed / Variable cabinet trade price | Listed F&O |
| 12 | PriceSpread | Price difference in the asset's convention (EP207) |
| 13–19 | ProductTicksIn… | Halves, fourths, eighths, 16ths, 32nds, 64ths, 128ths |
| 20 / 21 | Normal / Inverse rate representation | e.g. FX rate (EP100) |
| 22 | BasisPoints | Price in bp when not spread-based (EP161) |
| 23 | UpfrontPoints | CDS (EP161) |
| 24 | InterestRate | e.g. benchmark rate (EP174) |
| 25 | PercentageNotional | (EP208) |

Carried by 43 messages including `NewOrderSingle`, `ExecutionReport`,
`Quote`, `QuoteRequest`, `QuoteResponse`, `IOI`, `TradeCaptureReport`,
`AllocationInstruction`, `Confirmation`, `PositionReport`, the collateral
messages, `MarketData*` and `SecurityDefinition`/`SecurityList` (computed
from [1] by following component and group references). VERIFIED.

Note the name clash: `PriceType=1` "Percentage" means percent of par
(99.5), while the `Percentage` datatype is a fraction (0.995).

### `QtyType` (854)

| Value | Name | Requires |
|---|---|---|
| 0 | Units (shares, par, currency) | |
| 1 | Contracts | `ContractMultiplier` (231) |
| 2 | UnitsOfMeasurePerTimeUnit | `UnitOfMeasure` (996) and `TimeUnit` (997) |

Carried by 27 messages (orders, execution reports, trade capture,
allocations, confirmations, collateral, IOI, quote requests). VERIFIED [1].
`OrderQty` (38) is "the number of shares for equities or par, face or
nominal value for FI instruments", and `CashOrderQty` (152) is a quantity
in money. VERIFIED [1].

### Other convention fields

| Field | Meaning (from [1]) |
|---|---|
| `ContractMultiplier` (231) | Converts "nominal" units (contracts) to total units (shares). Quantities should in general be in the instrument's basic unit (shares, nominal, currency). |
| `PriceQuoteMethod` (1196) | STD (money per unit of a physical), INX (index), INT (interest-rate index), PCTPAR (percent of par). Part of the Instrument component. |
| `UnitOfMeasure` (996) / `UnitOfMeasureQty` (1147) | Underlying's unit (MWh, Bbl, t, IPNT index point, Ccy, ...) and how many per contract. |
| `PriceUnitOfMeasure` (1191) / `PriceUnitOfMeasureQty` (1192) | Unit the price is quoted in when it differs from the contract's, e.g. cattle: contract 40,000 lbs, price per 100 lbs. |
| `MinPriceIncrement` (969) / `MinPriceIncrementAmount` (1146) | Tick, and its money value (= tick × multiplier for listed derivatives). |
| `TickIncrement` (1208) | Tick for a price range (variable tick tables). |
| `RoundLot` (561), `MinTradeVol` (562) | Lot size, minimum order quantity. |
| `StrikeMultiplier` (967) | Applied to the strike for settlement value. |

All VERIFIED [1]. FIX has **no** equivalent of CME's `DisplayFactor`
(searched the repository; it is a CME user-defined tag). VERIFIED [1].

## 2. CME

**Wire encoding.** MDP 3.0 (schema version 13) defines `PRICE9` and
`PRICENULL9`: an int64 mantissa with a constant exponent of −9 that is not
sent on the wire; the null value is i64::MAX. Book, trade and strike prices
use them. `Decimal9`/`Decimal9NULL` (same layout) carry `DisplayFactor`,
`UnitOfMeasureQty` and `CouponRate`. `DecimalQty` is an int32 mantissa with
exponent −4 (used for `MinLotSize`, `LegOptionDelta`). VERIFIED [2]. These
types are marked `sinceVersion="9"`; older samples on the wiki show exponent
−7 (e.g. `969="5000000,-7"`). VERIFIED [2][3].

iLink 3 order entry: `Price` (44) is `PRICENULL9`, `OrderQty` (38) is
`uInt32`. VERIFIED [4]. A copy of the iLink 3 schema (v5, 2020) also has
`LastPx` as `PRICE9` and variable-exponent `Decimal32NULL`/`Decimal64NULL`
for option analytics (`Volatility`, `OptionDelta`, `RiskFreeRate`).
VERIFIED from a third-party mirror [5], not CME's own copy (CME
distributes it by SFTP).

**Quantities** are whole contracts: `OrderQty` uInt32, `MDEntrySize`
Int32. VERIFIED [2][4]. Exception: `MinLotSize` may be in units of measure
(e.g. megawatts) with 4 decimals when `LotType=4`. VERIFIED [2].

**`DisplayFactor` (9787).** Schema: "Contains the multiplier to convert the
CME Globex display price to the conventional price." VERIFIED [2]. Wiki:
"CME Globex Price × Display Factor = Display Price", likewise for ticks, and
it "should not be used for fractional prices". VERIFIED [3]. Examples [3]:

| Instrument | Globex price | DisplayFactor | Display price | Globex tick → display tick |
|---|---|---|---|---|
| ESH2 (2012 sample) | 113700 | 0.01 | 1137.00 | 25 → 0.25 |
| GEM2 (Eurodollar) | 9886.5 | 0.01 | 98.865 | 0.5 → 0.005 |
| SR3 (SOFR future) | 9820 (settlement) | 0.01 | 98.20 | 0.5 → 0.005 |
| SR3 put strike | 9575 | 0.01 (from underlying) | 95.75 | |

So for SOFR, the **venue** price is 9820, not 98.20. The ES row is an old
sample; whether ES still needs a factor today is UNVERIFIED (not checked
against a current secdef). Option strikes should use the underlying
future's factor, which CME "does not guarantee" for every option. VERIFIED
[3].

**Treasury futures.** Globex prices are decimal (e.g. 112.625); display is
fractional, driven by `MainFraction` (37702), `SubFraction` (37703) and
`PriceDisplayFormat` (9800): ZN 112.625 → `112'200`. VERIFIED [3][6]. The
2-year note tick is 0.00390625 (1/8 of 1/32); the conversion grid goes down
to 0.0078125. VERIFIED [7][8]. Oddities: products ticking in "modified
fourths" (30-day Fed Funds options, rough rice options) where the `.5` is
implied in display. VERIFIED [6].

**SOFR futures** (SR3): IMM Index = 100 − R; minimum fluctuation 0.005
($12.50), or 0.0025 ($6.25) for contracts with four months or less to
termination (Rule 46002.C). VERIFIED via the rulebook text quoted in
search results [9]; the rulebook PDF itself timed out, so re-check the
live text. Options on SR3 had tick changes filed in August 2025 (0.0025
for premiums ≤ 0.05 outside the nearest expiry) [10] (VERIFIED from filing
summary only). **Eurodollar** futures were converted to SOFR on 14 April
2023 by adding the ISDA fallback spread (26.161 bp) to final settlement
prices. VERIFIED [11]. That produced prices with 5 decimals, off the tick
grid.

## 3. ASX

**ASX Trade (equities)** prices are in **cents** on every interface:

- OUCH: 4-byte signed integer, "universally set to two decimal places"
  regardless of the instrument; example "BHP at 3676 cents" is sent as
  `367600`. VERIFIED [12].
- ITCH: same rule; examples "$0.235" → `2350`, "$0.005" → `50`. VERIFIED
  [13].
- FIX: "Prices will be disseminated in cents format i.e. a price entered as
  103.5 cents equates to $1.035", published to 4 decimal places; trade
  prices at an "extended price" (finer than the tick, e.g. Centre Point
  midpoint) always use 4 decimals. VERIFIED [14]. OUCH carries extended
  midpoint prices "with more than one decimal of a cent" as ASCII in an
  info field. VERIFIED [12].

The maintainer's claim (venue in cents, traders expect dollars) is
**confirmed**. Price steps: 0.1c below 10c, 0.5c from 10c to $2, 1c above.
VERIFIED [15][16] (the published text expresses the steps in cents and
dollars).

**ASX 24 (futures)** uses a different protocol: `Price` is a signed 64-bit
integer divided by a per-instrument **Price Fractional Denominator** from
the symbol directory (example: 97175000 / 1,000,000 = 97.175), with a
separate "Price Display Decimals". Quantities are whole lots. VERIFIED
[17]. Whether the denominator is always a power of ten is UNVERIFIED (the
spec only gives a decimal example).

Interest-rate futures quote 100 − yield: 90-day bank bills in multiples of
0.01; 3, 5, 10 and 20-year bond futures and the 30-day cash rate future in
multiples of 0.005 (3-year returned to 0.005 from 0.01 on 18 July 2025).
During the bond roll, spread ticks of 0.002 can leave legs at multiples of
0.001. VERIFIED [18][19]. Bond futures' tick value "varies with the level
of interest rates": value is computed from the yield with a bond formula,
not as price × multiplier. VERIFIED [18].

## 4. Equities generally

- **Quantity**: whole shares on exchange protocols (Nasdaq OUCH 5.0
  `Quantity` is an integer; ASX lots are integers). VERIFIED [20][12].
- **Round lots (US)**: tiered by price since the 2024 amendments: 100
  shares up to $250, 40 up to $1,000, 10 up to $10,000, 1 above. VERIFIED
  via exchange filings quoted in search results [21]; the SEC release [22]
  confirms the acceleration but not the tiers.
- **Fractional shares** at brokers: Fidelity 3 decimals ("rounded down to
  the nearest .001") [23]; Robinhood "as little as 0.000001 shares" (6
  decimals) [24]. VERIFIED. Other brokers not checked.
- **Tick regime (US)**: Rule 612 sets $0.01 at or above $1.00 and $0.0001
  below. VERIFIED [25]. A $0.005 tick for tick-constrained stocks was
  adopted in September 2024 [22]; its compliance date was deferred to
  November 2026 [26] and reportedly again to November 2027 (secondary
  source [27], UNVERIFIED against the SEC order).
- **Implied decimals**: Nasdaq OUCH 5.0 prices have 4 implied decimals in 8
  bytes, max $199,999.9900 [20]; TotalView-ITCH 5.0 has `Price (4)` fields
  and `Price (8)` for circuit-breaker levels [28]. VERIFIED.

## 5. Stress points for `Dec19`

- **Precision**: nothing found needs more than 19 decimals. CME is 9;
  Globex price × DisplayFactor (both 9 decimals) has at most 18, so the
  display price is exact. Binary fractions (1/256, 1/128) are exact.
- **Magnitude**: all wire fields found are ≤ 64-bit (PRICE9 max ≈ 9.2 ×
  10⁹ whole units). Nothing near 1.7 × 10¹⁹.
- **Not exactly decimal**: FIX inverse rates (`PriceType=21`), ASX 24's
  denominator if ever not a power of 2 or 5 (UNVERIFIED), and ASX bond
  futures' price-to-value conversion (a formula, not a multiplier).
- **Off-tick venue prices**: ASX extended/midpoint prices, ASX roll legs,
  converted Eurodollar prices. Prices must not be assumed to be tick
  multiples.
- **FIX text** allows any number of decimals by agreement, so a parser
  must reject values beyond 19 places (`Inexact`) rather than truncate.

## Implications for decimix / docs/units.md

- **"PRICE9" (to verify)**: confirmed for MDP 3.0 and iLink 3 (int64,
  exponent −9). Correct the wording "OUCH-style protocols typically use 4
  implied decimals": true for Nasdaq OUCH, but ASX Trade OUCH/ITCH use 2
  decimals **of a cent**, i.e. 4 decimals of a dollar, and ASX 24 uses a
  per-instrument denominator. `from_units(i128, decimals)` covers CME,
  Nasdaq and ASX Trade; ASX 24 needs a non-power-of-ten divisor or a check
  that the denominator is 10ⁿ.
- **ASX cents**: confirmed on OUCH, ITCH and FIX.
- **DisplayFactor (to verify)**: confirmed as "Globex price × factor =
  conventional price". Important for question 1: SOFR's venue price is
  9820, not 98.20, so the note's STIR example (95.25 means 4.75%) is the
  **display** price on CME.
- **Treasury 32nds**: confirmed at CME (decimal on the wire, fractional
  display from secdef tags). Other venues not checked here.
- **SOFR ticks (to verify)**: 0.005, or 0.0025 within four months of
  termination (bp: ½ and ¼). Eurodollar retired April 2023, confirmed.
- **Question 3 (FIX models this)**: confirmed. `PriceType` has 25 values,
  not only the ones listed; `QtyType` has 3 (units, contracts, units of
  measure per time unit). FIX's `ContractMultiplier` doc also backs the
  glossary's "bond quantity is face amount".
- **Glossary**: equities as whole shares plus broker fractions (3–6
  decimals seen) is confirmed.
- **Flag for the maintainer**: `decimix-finance` `Percentage` matches the
  FIX datatype (a fraction), but its doc mentions FX rates, which FIX types
  as `Price`/`float`. Worth a decision.
- **Still open**: whether ASX 24's denominator is always 10ⁿ; current ES
  DisplayFactor; the latest Rule 612 deferral.

## Sources

1. FIX Latest Orchestra repository, EP312: <https://github.com/FIXTradingCommunity/orchestrations/blob/master/FIX%20Standard/OrchestraFIXLatest.xml> (browsable at <https://orchimate.org/fixtrading/fix-latest>)
2. CME MDP 3.0 SBE schema v13, `templates_FixBinary.xml`: <ftp://ftp.cmegroup.com/SBEFix/Production/Templates/templates_FixBinary.xml>
3. CME Client Systems Wiki, MDP 3.0 – CME Globex Pricing: <https://cmegroupclientsite.atlassian.net/wiki/spaces/EPICSANDBOX/pages/457225869>
4. CME wiki, iLink New Order – Single: <https://cmegroupclientsite.atlassian.net/wiki/spaces/EPICSANDBOX/pages/715325687>
5. iLink 3 schema mirror (third party): <https://github.com/sambacha/CME-iLink3/blob/master/ilinkbinary.xml>
6. CME wiki, Fractional Pricing: <https://cmegroupclientsite.atlassian.net/wiki/spaces/EPICSANDBOX/pages/457092591>
7. CME wiki, Fractional Pricing – Examples for Order Entry: <https://cmegroupclientsite.atlassian.net/wiki/spaces/EPICSANDBOX/pages/457605865>
8. CME wiki, Fractional Pricing – Tick and Decimal Conversions: <https://cmegroupclientsite.atlassian.net/wiki/spaces/EPICSANDBOX/pages/457605881>
9. CME Rulebook Chapter 460, Three-Month SOFR Futures: <https://www.cmegroup.com/rulebook/CME/IV/400/460.pdf>
10. CME rule filing 25-343: <https://www.cmegroup.com/content/dam/cmegroup/market-regulation/rule-filings/2025/8/25-343.pdf>
11. CME press release, Eurodollar fallbacks conversion: <https://www.cmegroup.com/media-room/press-releases/2022/9/06/cme_group_proposesapril142023forfallbacksconversionofeurodollarf.html>
12. ASX Trade OUCH Message Specification v4.0 (April 2025): <https://www.asxonline.com/content/dam/asxonline/public/documents/asx-trade-refresh-manuals/asx-trade-ouch-message-specification.pdf>
13. ASX Trade ITCH Specification v3.4 (May 2026): <https://www.asxonline.com/content/dam/asxonline/public/documents/asx-trade-refresh-manuals/asx-trade-itch-message-specification.pdf>
14. ASX Trade FIX Order Entry Specification v1.4.1: <https://asxonline.com/content/dam/asxonline/public/documents/asx-trade-refresh-manuals/asx-trade-fix-order-entry-specification.pdf>
15. ASX equities trading: <https://www.asx.com.au/markets/trade-our-cash-market/asx-equities-trading>
16. ASX Operating Rules Procedures (13 Oct 2025): <https://www.asx.com.au/content/dam/asx/rules-guidance-notes-waivers/asx-operating-rules/rules/asx_or_procedures.pdf>
17. ASX 24 Market Data Protocol Specification v1.08: <https://www.asxonline.com/content/dam/asxonline/public/documents/NTP%20document%20library/Protocol%20technical%20specifications/asx-market-data-protocol-specification.pdf>
18. ASX 24 Contract Specifications v12: <https://www.asx.com.au/content/dam/asx/participants/derivatives-market/ird/asx24-contract-specifications.pdf>
19. ASX 3 Year Bond Futures tick change (July 2025): <https://www.asx.com.au/content/dam/asx/participants/derivatives-market/asx-3-year-treasury-bond-futures-tick-size-update-july-2025.pdf>
20. Nasdaq OUCH 5.0 (Oct 2025): <https://nasdaqtrader.com/content/technicalsupport/specifications/TradingProducts/Ouch5.0.pdf>
21. MIAX Pearl round-lot filing, Federal Register 2025-16697: <https://www.federalregister.gov/documents/2025/09/02/2025-16697>
22. SEC press release 2024-137: <https://www.sec.gov/newsroom/press-releases/2024-137>
23. Fidelity fractional shares: <https://www.fidelity.com/trading/fractional-shares>
24. Robinhood, 5 things to know about fractional shares: <https://robinhood.com/us/en/newsroom/5-things-to-know-about-fractional-shares/>
25. SEC Rule 612 FAQ: <https://www.sec.gov/divisions/marketreg/subpenny612faq.htm>
26. SEC exemptive order, Federal Register 2025-19926: <https://www.federalregister.gov/documents/2025/11/17/2025-19926>
27. TradeInformer on the 2027 extension: <https://tradeinformer.com/regulations/sec-extends-nms-relief-rule-611-repeal-2027>
28. Nasdaq TotalView-ITCH 5.0: <https://www.nasdaqtrader.com/content/technicalsupport/specifications/dataproducts/NQTVITCHspecification.pdf>
