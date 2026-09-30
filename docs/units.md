# Units: values as quoted

Status: **draft for review** (second round). Nothing here is implemented yet.

This note settles how prices and quantities relate to units: whose
convention a stored value is in, where conversions happen, and what to do
with values `Dec19` can't hold, such as large amounts in wei. It starts with
a glossary, because the rules are only as clear as the words they use.

Venue-specific details below are marked *(to verify)* where they come from
memory rather than a venue's specification.

## Glossary

### Units of an asset

**Asset unit**: the unit in which one whole of the traded thing is counted.
Order and position quantities are numbers of asset units (possibly
fractional). It depends on the asset class:

| Asset class | One asset unit is | Example quantity |
|---|---|---|
| Equities | one share | 100 shares; 0.25 of a share at brokers offering fractional shares |
| Fixed income (bonds) | one unit of the currency of **face value** (nominal) | EUR 1,000,000 nominal (the quantity is the face amount, not a number of bonds) *(to verify: some markets count bonds or denominations instead)* |
| Crypto | one coin or token (1 BTC, 1 ETH) | 0.5 ETH |
| FX | one unit of a currency; the order quantity is an amount of the pair's base currency | EUR 1,000,000 of EUR/USD |
| Futures and options (F&O) | one contract (lot) | 10 contracts |

**Base unit**: the smallest indivisible amount of an asset, used by some
systems as the counting unit instead: wei (10⁻¹⁸ ETH), satoshi (10⁻⁸ BTC).
The number of decimals between the asset unit and the base unit is fixed by
the asset (a token contract's `decimals`).

**Minor unit**: for a currency, the subdivision used for amounts (cents for
USD; 2 decimal places for most currencies, 0 for JPY, 3 for a few such as
KWD). A currency amount's precision, not a separate counting unit.

**Contract multiplier**: for F&O, how much of the underlying one contract
represents, or what one point of price is worth (e.g. an index future at
$50 per index point). It turns price × quantity into money. Instrument data,
not part of any stored value.

**Quantity step (lot size)** and **tick size**: the venue's smallest allowed
increments of quantity and price. Venue data, often much coarser than an
asset's decimals.

### Three kinds of price

The same price can be written three ways, and the rules below depend on
keeping them apart:

- **Wire value**: what a message on the wire carries. Often an integer with
  implied decimals: CME's order-entry and market-data interfaces send prices
  as an integer with 9 implied decimal places *(to verify: the "PRICE9"
  type)*; OUCH-style protocols typically use 4 implied decimals. Sometimes
  text, sometimes a fixed-point field.
- **Venue price**: the wire value decoded, in the venue's own convention:
  the units its rules, tick sizes and price limits are defined in. Decoding
  is exact and needs only the wire format (e.g. integer × 10⁻⁹).
- **Display price**: the convention traders expect to see and talk in, when
  it differs from the venue's:
  - ASX quotes in cents, while traders expect dollars (per the maintainer).
  - CME publishes a `DisplayFactor` per instrument to convert its price to
    the conventional one *(to verify: exact definition)*.
  - US Treasury futures are shown in 32nds (`110'16` is 110.5) though the
    venue price is a decimal *(to verify for each venue)*.
  - Short-term interest-rate futures (Eurodollar futures, since replaced by
    SOFR futures with the same convention) are priced at 100 minus the rate
    in percent: 95.25 means 4.75%, 0.01 of price is one basis point, and
    traders talk in basis points and ticks *(to verify: current tick sizes)*.

**Quoted value**, as used in this note, means the **venue price** (or venue
quantity): decoded from the wire, not converted to the display convention.

The same three levels apply to quantities, e.g. a venue that counts in
contracts where traders think in the underlying, or a chain that counts in
wei where people think in ETH.

**Decoding** is wire value → venue value. **Normalising** is any change of
unit or convention after that: venue price → display price, cents → dollars,
per-100 → per-1, "1000SHIB" contracts → SHIB, wei → ETH, one currency to
another.

## The problem

`Dec19` holds about ±1.7 × 10¹⁹ whole units with 19 decimal places. That is
plenty for any value in a sensible unit, but not for every unit:

- 100 ETH + 1 wei is `100.000000000000000001` in asset units (18 places,
  fits), but `100000000000000000001` in base units, which is 1 × 10²⁰ and
  doesn't fit. In wei a `Dec19` tops out at about 17 ETH.
- Some assets have more than 19 decimals (NEAR's base unit is 10⁻²⁴ NEAR),
  and some tokens have more than 1.7 × 10¹⁹ whole units in supply.

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
  (currency, per what quantity, index points, $/MWh, 100 − rate) is part of
  the instrument definition, not of the value.
- Converting to the display convention is presentation: it happens where
  values are shown to people, from instrument data, and is never written
  back.
- Code that combines values in different conventions (a notional from a
  price and a contract count, an exchange balance in ETH against an
  on-chain balance in wei) applies the multiplier or decimals at that point,
  explicitly, as a calculation rather than as a change to stored data.

`Dec19` suits this because it never needs a multiplier to *hold* a quoted
value: 19 decimal places and ±1.7 × 10¹⁹ cover venue prices and almost all
venue quantities as they come. Every 32nd, 64th or 128th is an exact decimal
(1/128 = 0.0078125), so fractional conventions convert exactly. And because
each value has one representation, strike prices match exactly by value:
`105.5` and `105.50` are the same `Dec19`, where as strings they wouldn't
match.

## Prices

- Stored as venue prices, in `Dec19`.
- The display convention is applied only for display.
- Strikes are `Dec19` venue prices; matching instruments by strike is exact
  equality.

## Quantities

- Stored as venue quantities.
  - Where a venue counts asset units (e.g. `0.00100000` ETH; I believe most
    centralised crypto exchanges do this, but haven't checked each), that is
    `Dec19`/`UDec19`, down to 1 wei.
  - Contract and lot counts are whole numbers, as quoted.
- Where a source counts whole **base units** (wei, satoshi, yoctoNEAR),
  typically on-chain data, custody and some settlement APIs, the quoted
  value is an integer that may not fit a `Dec19`. It is stored as an
  integer: a `u128` holds 3.4 × 10³⁸ base units, far beyond any real supply
  (all ETH is about 1.2 × 10²⁶ wei). No 256-bit arithmetic and no
  BigDecimal: the integer is already exact.
- Where the two meet (reconciling an exchange balance with an on-chain one,
  settlement), the conversion is an explicit calculation using the asset's
  decimals.

## What decimix needs

Exact conversions between a decimal and an integer with implied decimals.
They serve two different jobs, both at the edges:

- **Wire encoding and decoding** for interfaces that carry prices or
  quantities as integers with implied decimals, e.g. CME order entry
  (9 implied decimals, *to verify*) or OUCH-style protocols (4). Sending
  a price the wire can't represent exactly is a bug, so these must be
  exact by default.
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
`to_scaled` with a wider result.

decimix itself doesn't need a base-units type: a plain `u128`/`i128` is the
quoted value. A domain newtype (e.g. `BaseQty` in `decimix-finance`) would
stop base-unit quantities being mixed with asset-unit ones by accident; see
the questions.

## Limits and exceptions

| Case | Example | Handling |
|---|---|---|
| Asset units with more than 19 decimals | NEAR (24) | Keep base units (`u128`) for exact amounts; round dust with a named mode only where the venue's precision is coarser anyway |
| More than 1.7 × 10¹⁹ whole tokens | some very large-supply tokens | Venues typically quote these in multiples (e.g. "1000SHIB"-style contracts); store as quoted. Otherwise base units |
| "Unlimited" on-chain amounts | ERC-20 approvals of `uint256::MAX` | A sentinel, not a quantity: anything ≥ 2¹²⁸ at the edge is reported as out of range or mapped to an explicit "unlimited" |

All three show up loudly at the edge (`OutOfRange` or `Inexact`), never as a
silently wrong value. The examples illustrate each pattern; which assets and
venues fall into each case needs checking per venue.

## Questions for review

1. **The brief says "fold venue display factors in at ingest".** With the
   glossary above, folding CME's `DisplayFactor` in at ingest converts venue
   prices to display prices, i.e. normalises, which this note rules out for
   stored data. Which is intended: store venue prices (this note; the brief
   would change), or store display prices (the brief; this note would
   change)?
2. **Base-unit quantities**: plain `u128`/`i128`, or a newtype that can't be
   mixed with asset-unit quantities (e.g. `BaseQty` in `decimix-finance`,
   converting only via `from_units`/`to_units` with the asset's decimals)?
3. **Persisted data and its convention**: a stored value only means
   something together with its instrument's convention, and conventions
   change (redenominations, new contract multipliers). Should persisted
   history record the convention (or the instrument-definition version) it
   was stored under, as `DecVec` records its scale?
4. **Names**: `from_units`/`to_units` (and `_round`), replacing
   `from_scaled`/`to_scaled`.
5. **Dust**: is rounding sub-10⁻¹⁹ dust at ingest ever acceptable (with a
   named mode), or must such assets always stay in base units?
6. **Glossary**: are these the right definitions for your systems,
   especially for fixed income (face amount vs number of bonds) and FX
   (base-currency amount)?
