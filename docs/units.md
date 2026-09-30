# Units: values as quoted

Status: **draft for review** (third round: glossary and examples checked
against the research in [`research/`](research/README.md)). Nothing here is
implemented yet.

This note settles how prices and quantities relate to units: whose
convention a stored value is in, where conversions happen, and what to do
with values `Dec19` can't hold, such as large amounts in wei. It starts with
a glossary, because the rules are only as clear as the words they use.

Claims about venues and markets come from the research notes, which cite
primary sources: [listed markets and FIX](research/listed-and-fix.md),
[fixed income and rates](research/fixed-income-and-rates.md),
[FX and crypto](research/fx-and-crypto.md). What they couldn't verify is
marked *(unverified)* here.

## Glossary

### Units of an asset

**Asset unit**: the unit in which one whole of the traded thing is counted.
Order and position quantities are numbers of asset units (possibly
fractional). It depends on the asset class:

| Asset class | One asset unit is | Example quantity |
|---|---|---|
| Equities | one share | 100 shares on exchange (whole shares); brokers offering fractional shares go to 3–6 decimals |
| Fixed income (bonds) | one unit of the currency of **face value** (nominal) | EUR 1,000,000 nominal: the quantity is the face amount, not a number of bonds (gilts, Treasuries, Bunds, BTPs, JGBs). Some venues count lots of fixed notional instead (BrokerTec Chicago: $100k/$200k), and exchange-traded notes count units |
| FX | one unit of a currency; the order quantity is an amount of the **dealt currency**, usually the pair's base currency but it can be the terms currency | EUR 1,000,000 of EUR/USD, or USD 1,000,000 of it. The quantity's currency must be stored with it (FIX `Currency` (15)) |
| Crypto | one coin or token (1 BTC, 1 ETH) | 0.5 ETH, sent by exchanges as decimal strings. Variants: market orders sized in the quote currency (e.g. Binance `quoteOrderQty`); derivatives counted in contracts, in USD, or in coins; synthetic multiples ("1000PEPE", "1000000MOG") as the asset |
| Futures and options (F&O) | one contract (lot) | 10 contracts (whole numbers; CME sends them as unsigned 32-bit integers) |

**Base unit**: the smallest indivisible amount of an asset, used by some
systems as the counting unit instead: wei (10⁻¹⁸ ETH), satoshi (10⁻⁸ BTC),
lamport (10⁻⁹ SOL), yoctoNEAR (10⁻²⁴ NEAR). The number of decimals between
the asset unit and the base unit is fixed by the asset. For ERC-20 tokens it
is the contract's `decimals`, anything from 0 to 255: 18 is common, but 2
(GUSD) and 24 (YAM-V2) exist.

**Minor unit**: for a currency, the subdivision used for amounts (cents for
USD). ISO 4217 gives each currency's number of decimals: 2 for most, 0 for
17 currencies (e.g. JPY), 3 for 7 (e.g. KWD), and 4 for two (CLF, UYW).

**Pip**: the conventional price step of an FX pair, and instrument data like
a tick size. Usually the 4th decimal, but the 2nd for JPY, HUF and THB pairs
and the 3rd for CZK pairs; venues often quote a fraction of a pip.

**Contract multiplier**: for F&O, how much of the underlying one contract
represents, or what one point of price is worth (e.g. an index future at
$50 per index point). It turns price × quantity into money. Instrument data,
not part of any stored value. (ASX 24 bond futures are an exception: their
money value comes from the yield by a formula, not price × multiplier.)

**Factor**: for some bonds, a number between quantity and money: a
mortgage-backed security's **pool factor** (8 decimals at Fannie Mae), or an
inflation-linked bond's **index ratio** (5 decimals for gilts, TIPS and
JGBi). In FIX terms, money = quantity × factor × price. Instrument data,
like the contract multiplier.

**Price per 100 nominal** (percent of par): how most bonds are priced, e.g.
98.765 means 98.765% of face value (FIX `PriceType` 1). **Clean price**
excludes accrued interest; **dirty price** includes it. **Accrued interest**
is a money amount computed from the quantity and a day-count fraction, and
rounded once, to the currency's minor unit.

**Quantity step (lot size)** and **tick size**: the venue's smallest allowed
increments of quantity and price. Venue data, usually much coarser than an
asset's decimals. Stored prices aren't always on the tick grid (see *The
principle*).

### Three kinds of price

The same price can be written three ways, and the rules below depend on
keeping them apart:

- **Wire value**: what a message on the wire carries. Often an integer with
  implied decimals:
  - CME (MDP 3.0 market data and iLink 3 order entry): a 64-bit integer
    with 9 implied decimals (`PRICE9`; older schemas used 7).
  - Nasdaq OUCH: 4 implied decimals.
  - ASX Trade (OUCH, ITCH): 2 implied decimals **of a cent**, e.g. a price
    of 3676 cents is sent as `367600`.
  - ASX 24: a per-instrument "price fractional denominator" *(unverified:
    whether it is always a power of ten)*.
- **Venue price**: the wire value decoded, in the venue's own convention:
  the units its rules, tick sizes and price limits are defined in. Decoding
  is exact and needs only the wire format.
- **Display price**: the convention traders expect to see and talk in, when
  it differs from the venue's:
  - ASX quotes in **cents** on all its ASX Trade interfaces (OUCH, ITCH
    and FIX), while traders expect dollars.
  - CME publishes a `DisplayFactor` (tag 9787) per instrument:
    Globex price × `DisplayFactor` = conventional price.
  - **SOFR futures at CME**: the venue price is `9820`, which displays as
    **98.20**, meaning 100 − 98.20 = a 1.80% rate. Traders talk in basis
    points and ticks (0.005, or 0.0025 within four months of expiry
    *(unverified: from rulebook text quoted in search results)*).
    Eurodollar futures, which used the same convention, were converted to
    SOFR on 14 April 2023.
  - US Treasury futures at CME are decimal on the wire (112.625) and shown
    in 32nds (`112'200`), from fields in the security definition.

FIX has no field like CME's `DisplayFactor`; its `PriceType` (below) says
what kind of price a value is, not how to display it.

**Quoted value**, as used in this note, means the **venue price** (or venue
quantity): decoded from the wire, not converted to the display convention.

The same three levels apply to quantities, e.g. a venue that counts in
contracts where traders think in the underlying, or a chain that counts in
wei where people think in ETH.

**Decoding** is wire value → venue value. **Normalising** is any change of
unit or convention after that: venue price → display price, cents → dollars,
per-100 → per-1, "1000SHIB" contracts → SHIB, wei → ETH, one currency to
another.

### What kind of price: FIX `PriceType`

FIX's `PriceType` (tag 423) has 25 values, including: per unit, percentage
of par, yield, spread, basis points, upfront points, interest rate, FX rate
(normal or inverse), and counts of ticks in halves through 128ths. It
appears in 43 FIX messages. `QtyType` (tag 854) has three values: units,
contracts, and units of measure per time unit. A price's number alone
doesn't say which of these it is.

Note a naming clash: the FIX *datatype* `Percentage` is a fraction (0.05 is
5%), while `PriceType` 1, also called "Percentage", means percent of par
(99.5).

## The problem

`Dec19` holds about ±1.7 × 10¹⁹ whole units with 19 decimal places. That is
plenty for any value in a sensible unit (the research found nothing stored
that needs more than 10 decimals, or anywhere near that range), but not for
every unit:

- 100 ETH + 1 wei is `100.000000000000000001` in asset units (18 places,
  fits), but `100000000000000000001` in base units, which is 1 × 10²⁰ and
  doesn't fit. In wei a `Dec19` tops out at about 17 ETH.
- Some assets have more than 19 decimals (NEAR and YAM-V2 have 24).

The obvious fix, normalising everything to one "natural" unit at ingest, has
a known failure mode, from experience on a system that normalised prices
internally and converted back at the edges:

- the conversion is easy to forget, in one direction or the other, at one
  of many edges;
- the data needed to do it (multipliers, conventions) isn't always
  available where the conversion has to happen;
- for many instruments there is no natural unit to convert to: futures and
  commodities priced in kWh, weather index points or offsets; interest-rate
  futures at 100 minus a rate; strike prices, which are used almost like
  strings for instrument matching.

## The principle

**Store quoted values: decode at the edge, never normalise.**

- Each venue's parser decodes wire values into venue values, exactly, once.
  That needs only the wire format.
- Stored values stay in the venue's convention. What a value means
  (currency, per what quantity, index points, $/MWh, 100 − rate, percent of
  par, yield) is part of the instrument definition or recorded with the
  value (see question 3), not assumed.
- Converting to the display convention is presentation: it happens where
  values are shown to people, from instrument data, and is never written
  back.
- Code that combines values in different conventions (a notional from a
  price and a contract count, an exchange balance in ETH against an
  on-chain balance in wei) applies the multiplier or decimals at that point,
  explicitly, as a calculation rather than as a change to stored data.
- Don't assume stored prices are on the tick grid. Venue prices can fall
  between ticks (ASX midpoint prices, roll legs, and the Eurodollar-to-SOFR
  conversion, which added 26.161 basis points).
- Reject text with more than 19 decimal places rather than truncating it
  (FIX text can carry more): the parser already does, with `TooPrecise`.

`Dec19` suits this because it never needs a multiplier to *hold* a quoted
value. Every 32nd, 64th, 128th and 512th is an exact decimal
(1/512 = 0.001953125). CME's price × `DisplayFactor` has at most 18
decimals, so even the display conversion is exact. And because each value
has one representation, strike prices match exactly by value: `105.5` and
`105.50` are the same `Dec19`, where as strings they wouldn't match.

## Prices

- Stored as venue prices, in `Dec19`: `9820` for that SOFR future, cents for
  ASX Trade.
- The display convention is applied only for display.
- Strikes are `Dec19` venue prices; matching instruments by strike is exact
  equality.

## Quantities

- Stored as venue quantities, with their currency where it can vary (FX).
  - Where a venue counts asset units (crypto spot exchanges send decimal
    strings of at most 10 decimals in the research), that is
    `Dec19`/`UDec19`, down to 1 wei.
  - Contract and lot counts are whole numbers, as quoted.
- Where a source counts whole **base units** (wei, satoshi, yoctoNEAR),
  typically on-chain data, custody and some settlement APIs, the quoted
  value is an integer that may not fit a `Dec19` (SHIB's supply is 10³³
  base units). It is stored as an integer: a `u128` holds 3.4 × 10³⁸ base
  units, far beyond any real supply. No 256-bit arithmetic and no
  BigDecimal: the integer is already exact.
- Where the two meet (reconciling an exchange balance with an on-chain one,
  settlement), the conversion is an explicit calculation using the asset's
  decimals.

## What decimix needs

**Exact conversions between a decimal and an integer with implied
decimals.** They serve two jobs, both at the edges:

- **Wire encoding and decoding** for interfaces that carry values as
  integers with implied decimals: CME (9), Nasdaq OUCH (4), ASX Trade (2
  decimals of a cent, i.e. 4 of a dollar, on a cents price). Sending a
  price the wire can't represent exactly is a bug, so these must be exact
  by default.
- **Base units** (wei and so on) where conventions meet, as above.

Today's `from_scaled`/`to_scaled` only take an `i64`, which is too small for
base units (at most about 9.2 ETH in wei), and `to_scaled` always rounds.

```rust
impl Dec19 {
  /// `units × 10^-decimals`, exactly: e.g. 113725000000 with 9 decimals is
  /// 113.725, and 100000000000000000001 (wei) with 18 decimals is
  /// 100.000000000000000001. Fails if the value would need more than 19
  /// decimal places (`Inexact`) or is out of range (`OutOfRange`).
  pub fn from_units(units: i128, decimals: u32) -> Result<Dec19, UnitsError>;

  /// The reverse: this value as an integer with `decimals` implied
  /// decimals, exactly, or an error if it has more decimal places than that
  /// (`Inexact`) or doesn't fit (`OutOfRange`). For a narrower wire field,
  /// convert the result with `i64::try_from`.
  pub fn to_units(self, decimals: u32) -> Result<i128, UnitsError>;

  /// Rounding variants, naming what happens to the extra digits: for
  /// inputs finer than 10⁻¹⁹, and for deliberately rounding to a coarser
  /// wire precision.
  pub fn from_units_round(units: i128, decimals: u32, mode: Round)
    -> Result<Dec19, OutOfRange>;
  pub fn to_units_round(self, decimals: u32, mode: Round)
    -> Result<i128, OutOfRange>;
}
// UDec19: the same with u128.
// Also a text form: whole-number text with implied decimals, for APIs that
// send base units as strings.
```

These replace `from_scaled`/`to_scaled`: `to_units_round` is today's
`to_scaled` with a wider result. If ASX 24's denominators turn out not to be
powers of ten, it needs a division instead (`div_int` covers it).

decimix itself doesn't need a base-units type: a plain `u128`/`i128` is the
quoted value. A domain newtype (e.g. `BaseQty` in `decimix-finance`) would
stop base-unit quantities being mixed with asset-unit ones by accident; see
the questions.

**Rounding rules found in the research map onto decimix's six modes**, with
one gap:

| Rule | decimix mode |
|---|---|
| DMO "nearest", ISDA ".005 rounded upwards" | `HalfAwayFromZero` (for positive values) |
| ISDA JPY/KRW "round down"; "rounded down to 4 decimal places" | `TowardZero` (`Floor` for positive values) |
| Currencies a venue truncates (e.g. FXall `RoundDownCcy`) | `TowardZero` |
| TIPS: truncate the index ratio to 6 places, then round to 5 | one `HalfAwayFromZero` round to 5 places (not the same as `HalfEven`) |
| "No intermediate rounding" (DMO, US Treasury rules) for day-count fractions and interest | **gap**: needs a fused multiply-then-divide with one rounding (question 7) |

## Limits and exceptions

| Case | Example | Handling |
|---|---|---|
| Asset units with more than 19 decimals | NEAR, YAM-V2 (24) | Keep base units (`u128`) for exact amounts; round dust with a named mode only where the venue's precision is coarser anyway |
| Base-unit integers beyond `Dec19` | SHIB's supply, 10³³ base units | Store as `u128`/`i128`, as quoted |
| Very large supplies in whole tokens | none found above 4.2 × 10¹⁷ (BabyDoge), well inside the range | No special handling needed. Venues' "1000PEPE"-style contracts exist to give readable prices and ticks, not for range; store them as quoted |
| "Unlimited" on-chain approvals | `uint256::MAX`, but also 2⁹⁶ − 1 for UNI and COMP (7.9 × 10²⁸, which fits a `u128`) | A sentinel, not a quantity, and **no magnitude threshold identifies it**. Recognise each token's sentinel explicitly and map it to an "unlimited" value |
| Raw DEX pool prices (base unit per base unit) | a $10⁻⁸ token with 18 decimals priced in 6-decimal USDC is 10⁻²⁰ | Below `Dec19`'s smallest step: price in asset units instead (10⁻⁸), per the principle |

All of these show up loudly at the edge (`OutOfRange` or `Inexact`), never
as a silently wrong value.

## Questions for review

1. **The brief says "fold venue display factors in at ingest".** Folding
   CME's `DisplayFactor` in at ingest converts venue prices to display
   prices, i.e. normalises, which this note rules out for stored data.
   Concretely: is a SOFR future stored as `9820` (this note; the brief
   would change) or as `98.20` (the brief; this note would change)?
2. **Base-unit quantities**: plain `u128`/`i128`, or a newtype that can't be
   mixed with asset-unit quantities (e.g. `BaseQty` in `decimix-finance`,
   converting only via `from_units`/`to_units` with the asset's decimals)?
3. **Persisted data and its convention**: a stored value only means
   something together with its convention, and conventions change
   (redenominations, new contract multipliers). Should persisted data
   record the convention it was stored under, as `DecVec` records its
   scale? FIX already models this: a stored price could carry its
   `PriceType` (or the instrument definition fixes it) and a quantity its
   `QtyType` and currency. That also covers RFQs where one dealer quotes a
   bond on price and another on yield. decimix itself stays
   convention-free; this belongs to the domain layer.
4. **Names**: `from_units`/`to_units` (and `_round`), replacing
   `from_scaled`/`to_scaled`.
5. **Dust**: is rounding sub-10⁻¹⁹ dust at ingest ever acceptable (with a
   named mode), or must such assets always stay in base units?
6. **Glossary**: do these definitions match your systems? The FX and crypto
   rows are the ones most worth a second opinion.
7. **Fused multiply-then-divide** (e.g. `mul_div(a, b, c, Round)`, one
   rounding): needed for day-count and accrued-interest formulas, which
   the DMO and US Treasury rules say must not round in between. decimix has
   `mul` and `div` but no fused form today.
8. **"Round half up"**: document `HalfAwayFromZero` as decimix's reading of
   "nearest" and "round half up" in DMO, Treasury and ISDA texts? For
   negative rates the ISDA wording is ambiguous (its examples are all
   positive).
9. **`decimix-finance` `Percentage`**: it matches the FIX datatype (a
   fraction), but its doc comment mentions FX rates, which FIX types as
   `Price` (or `float`). Should FX rates be `Price` instead?
