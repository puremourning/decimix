# Units: values as quoted

Status: **draft for review**. Nothing here is implemented yet.

This note settles how prices and quantities relate to units: whose
convention a stored value is in, where conversions happen, and what to do
with values `Dec19` can't hold, such as large amounts in wei.

## The problem

`Dec19` holds about ±1.7 × 10¹⁹ whole units with 19 decimal places. That is
plenty for any value in a sensible unit, but not for every unit:

- 100 ETH + 1 wei is `100.000000000000000001` in ETH (18 places, fits), but
  `100000000000000000001` in wei, which is 1 × 10²⁰ and doesn't fit. In wei
  a `Dec19` tops out at about 17 ETH.
- Some assets have more than 19 decimals (NEAR's smallest unit is 10⁻²⁴),
  and some tokens have more than 1.7 × 10¹⁹ whole units in supply.

The obvious fix, converting everything to one "natural" unit at ingest, has
a known failure mode, from experience on a system that normalised prices
internally and converted back at the edges:

- the conversion is easy to forget, in one direction or the other, at one
  of many edges;
- the data needed to do it (multipliers, conventions) isn't always
  available where the conversion has to happen;
- for many instruments there is no natural unit to convert to: futures and
  commodities priced in kWh, weather index points or offsets; bond futures
  in 32nds; strike prices, which are used almost like strings for
  instrument matching.

## The principle

**Store values as their source quotes them, decoded exactly. Don't
normalise units.**

Two different operations are easy to conflate:

- **Decoding** turns a venue's wire encoding into the value the venue
  quotes: an integer times a display factor (CME's `DisplayFactor`), implied
  decimals, a fractional format such as 32nds (`110'16` is 110.5). It
  happens once, in the venue's parser, and needs only data the parser
  already has. After decoding, the value means exactly what the venue says
  it means.
- **Normalising** changes the unit: cents to dollars, per-100 to per-1,
  "1000SHIB" contracts to SHIB, wei to ETH, one currency to another. This is
  the step that gets forgotten and needs data that isn't always there.

The rule: decode at the edge, never normalise stored data. A value's unit is
whatever its source uses, and the instrument (or asset) definition says what
that is. Code that combines values in different conventions, such as a
notional from a per-contract price and a lot count, applies the multiplier
at that point, explicitly, as a calculation rather than as a change to
stored data.

`Dec19` suits this because it never needs a multiplier to *hold* a quoted
value: 19 decimal places and ±1.7 × 10¹⁹ cover every venue's quoted prices
and almost every quoted quantity as they come. Every 32nd, 64th or 128th is
an exact decimal (1/128 = 0.0078125), so fractional formats decode exactly.
And because each value has one representation, strike prices match exactly
by value: `105.5` and `105.50` are the same `Dec19`, where as strings they
wouldn't match.

## Prices

- Stored as quoted, in `Dec19`, after decoding. No conversion to a major
  currency unit, no per-unit normalisation.
- What a price means (currency, per what quantity, index points, $/MWh) is
  part of the instrument definition, not of the value.
- Strikes are `Dec19` as quoted; matching instruments by strike is exact
  equality.

## Quantities

- Stored as quoted as well.
  - Where a venue quotes in asset units (e.g. `0.00100000` ETH; I believe
    most centralised exchanges do this, but haven't checked each), that is
    `Dec19`/`UDec19`, down to 1 wei.
  - Contract and lot counts are whole numbers, as quoted.
- Where a source quotes whole **base units** (wei, satoshi, yoctoNEAR),
  typically on-chain data, custody and some settlement APIs, the quoted
  value is an integer that may not fit a `Dec19`. It is stored as an
  integer: a `u128` holds 3.4 × 10³⁸ base units, far beyond any real
  supply (all ETH is about 1.2 × 10²⁶ wei). No 256-bit arithmetic and no
  BigDecimal: the integer is already exact.
- Where the two conventions meet (reconciling an exchange balance in ETH
  with an on-chain balance in wei, or settlement), the conversion is an
  explicit calculation using the asset's decimals, which a token contract
  fixes forever. This is the "combine at the point of use" case above, not
  a normalisation of stored data.

## What decimix needs

Exact conversions between a decimal and a whole number of base units, for
the places where the two conventions meet. Today's `from_scaled` only takes
an `i64` (at most about 9.2 ETH in wei), which is too small.

```rust
impl Dec19 {
  /// `units × 10^-decimals`, exactly: e.g. 100000000000000000001 wei with
  /// 18 decimals is 100.000000000000000001. Fails if `decimals` is more
  /// than 19 and the value would need more places (`Inexact`), or if the
  /// result is out of range (`OutOfRange`).
  pub fn from_units(units: i128, decimals: u32) -> Result<Dec19, UnitsError>;

  /// The reverse: this value as a whole number of base units with
  /// `decimals` decimals, exactly, or an error if it has more places than
  /// that (`Inexact`) or doesn't fit (`OutOfRange`).
  pub fn to_units(self, decimals: u32) -> Result<i128, UnitsError>;

  /// Rounding variants for assets with more than 19 decimals, where the
  /// caller decides what happens to dust smaller than 10⁻¹⁹.
  pub fn from_units_round(units: i128, decimals: u32, mode: Round)
    -> Result<Dec19, OutOfRange>;
}
// UDec19: the same with u128.
// Also a text form: whole-number text with implied decimals, for APIs that
// send base units as strings.
```

These subsume `from_scaled`/`to_scaled`, which could then be removed or
become the same functions.

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
silently wrong value. The examples are illustrations of each pattern; which
assets and venues fall into each case needs checking per venue.

## Questions for review

1. **The brief says "fold venue display factors in at ingest".** This note
   reads that as *decoding* (integer × display factor → the quoted price),
   which is compatible with "store as quoted". If that's the intent, the
   brief should say so explicitly; if it meant normalising, this note
   reverses it.
2. **Base-unit quantities**: plain `u128`/`i128`, or a newtype that can't be
   mixed with asset-unit quantities (e.g. `BaseQty` in `decimix-finance`,
   converting only via `from_units`/`to_units` with the asset's decimals)?
3. **Persisted data and its convention**: a stored value only means
   something together with its instrument's convention, and conventions
   change (redenominations, new contract multipliers). Should persisted
   history record the convention (or the instrument-definition version) it
   was stored under, as `DecVec` records its scale?
4. **Names**: `from_units`/`to_units`, and whether they replace
   `from_scaled`/`to_scaled`.
5. **Dust**: is rounding sub-10⁻¹⁹ dust at ingest ever acceptable (with a
   named mode), or must such assets always stay in base units?
