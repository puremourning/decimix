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
  `from_scaled`/`to_scaled`.
- Explicit float boundary: `to_f64_lossy` and `from_f64_lossy(x, step, Round)`.
- `bigdecimal` feature: exact `From<Dec19> for BigDecimal`, `to_big`,
  `from_big`, `TryFrom<&BigDecimal>`.
- `ProductSum` for exact sums of products; `newtype!` for domain types.
- `decimix-finance` crate (unpublished): `Price`, `Qty` (unsigned quantity),
  `DeltaQty` (signed quantity), `Amt` (price × quantity), `Percentage`, and
  the `Quantity` trait over both quantity forms.
