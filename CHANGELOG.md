# Changelog

This file describes notable changes in major and minor releases with a focus
solely on incompatible or API changes. For the full changelog see the git log.

## [Unreleased]

### Added

- Initial release
- `Dec19` (signed) and `UDec19` (unsigned): 19-decimal-place fixed-point
  types with exact operators, `checked_*`/`saturating_*` methods, and
  rounding methods that take a mandatory `Round`.
- `Round` with six modes: `HalfEven`, `HalfAwayFromZero`, `Floor`, `Ceiling`,
  `TowardZero`, `AwayFromZero`.
- `dec!`/`udec!` compile-time literals; exact text and ASCII in/out
  (`from_ascii`, `write_ascii`, `write_ascii_dp`, `to_ascii`, `FromStr`,
  `Display`).
- Raw and wire access: `to_raw`/`from_raw`, `to_le_bytes`/`from_le_bytes`,
  `from_scaled`/`to_scaled`, and order-preserving
  `to_key_bytes`/`from_key_bytes` for sorted storage.
- Explicit float boundary: `to_f64_lossy` and `from_f64_lossy(x, step, Round)`.
- `bigdecimal` feature: exact `From<Dec19> for BigDecimal`, `to_big`,
  `from_big`, `TryFrom<&BigDecimal>`.
- Rounding multiplication and division (`mul`, `div` and their `checked_`
  and `saturating_` forms); `Dec19` accepts a `UDec19` operand.
- Exact Euclidean division for lot counts and grids, named as in std:
  `div_euclid`, `rem_euclid` and their `checked_` forms.
- `%` and `%=` (remainder with the sign of the left-hand side, as on Rust's
  integers and `f64`) and `checked_rem`, on both types and on `newtype!`
  types.
- `Dec19::unsigned_abs` and `Dec19::abs_diff`, both returning `UDec19` and
  never overflowing.
- `ProductSum` for exact sums of products (`add`, `checked_add`, `finish`,
  `div`); `newtype!` for domain types.
- `capnp-decimix` (not yet published): Cap'n Proto encoding as two `UInt64`
  words, with `get_dec19`/`set_dec19` and `get_udec19`/`set_udec19` on field
  readers and builders, and `import_path()`/`SCHEMA_ID` for a `build.rs` to
  import `decimix.capnp`. Needs the capnproto-rust `newtype` branch.
- `decimix-finance` crate (unpublished): `Price`, `Qty` (unsigned quantity),
  `DeltaQty` (signed quantity), `Amt` (price × quantity), `Percentage`, and
  the `Quantity` trait over both quantity forms.
