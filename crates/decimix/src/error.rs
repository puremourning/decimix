//! Error types.

use core::fmt;

/// Why a piece of text could not be read as a decimal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseError {
  /// Not a plain decimal number: empty, a stray character, a lone sign or
  /// point, an exponent, and so on.
  Invalid,
  /// The number has non-zero digits beyond the 19th decimal place. Use a
  /// `*_round` function to round it instead.
  TooPrecise,
  /// The number is too large (or, for an unsigned type, negative).
  OutOfRange,
}

impl ParseError {
  /// A short description, usable in `const` contexts (the `dec!` macro turns
  /// it into a compile error).
  #[must_use]
  pub const fn as_str(self) -> &'static str {
    match self {
      ParseError::Invalid => "invalid decimal number",
      ParseError::TooPrecise => "more than 19 decimal places",
      ParseError::OutOfRange => "decimal number out of range",
    }
  }
}

impl fmt::Display for ParseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self.as_str())
  }
}

impl core::error::Error for ParseError {
}

/// The result does not fit in the target type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutOfRange;

impl fmt::Display for OutOfRange {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("value out of range")
  }
}

impl core::error::Error for OutOfRange {
}

/// The output buffer is too short for the formatted number.
///
/// A buffer of the type's `MAX_ASCII_LEN` bytes is always enough.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferTooSmall;

impl fmt::Display for BufferTooSmall {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("output buffer too small")
  }
}

impl core::error::Error for BufferTooSmall {
}

/// Why an `f64` could not be converted to a decimal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FromF64Error {
  /// The double is NaN or infinite.
  NotFinite,
  /// The double is too large (or, for an unsigned type, negative).
  OutOfRange,
}

impl fmt::Display for FromF64Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(match self {
      FromF64Error::NotFinite => "f64 is NaN or infinite",
      FromF64Error::OutOfRange => "f64 out of range",
    })
  }
}

impl core::error::Error for FromF64Error {
}

/// Why a `BigDecimal` could not be converted exactly.
#[cfg(feature = "bigdecimal")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TryFromBigError {
  /// The value has non-zero digits beyond the 19th decimal place. Use
  /// `from_big` with a [`Round`](crate::Round) to round it instead.
  Inexact,
  /// The value is too large (or, for an unsigned type, negative).
  OutOfRange,
}

#[cfg(feature = "bigdecimal")]
impl fmt::Display for TryFromBigError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(match self {
      TryFromBigError::Inexact => "more than 19 decimal places",
      TryFromBigError::OutOfRange => "value out of range",
    })
  }
}

#[cfg(feature = "bigdecimal")]
impl core::error::Error for TryFromBigError {
}
