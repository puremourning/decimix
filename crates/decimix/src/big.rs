//! Exact conversions to and from `bigdecimal::BigDecimal` (feature
//! `bigdecimal`).
//!
//! `BigDecimal` is the slow path for maths this crate doesn't do (square roots,
//! high-precision analytics). Going to `BigDecimal` is always exact. Coming
//! back either names its rounding (`from_big`) or refuses to round
//! (`TryFrom`).

use bigdecimal::BigDecimal;
use bigdecimal::num_bigint::BigInt;

use crate::{Dec19, OutOfRange, Round, TryFromBigError, UDec19};

macro_rules! impl_big {
  ($T:ident, $mac:literal) => {
    impl From<$T> for BigDecimal {
      /// Always exact.
      fn from(v: $T) -> BigDecimal {
        BigDecimal::new(BigInt::from(v.to_raw()), 19)
      }
    }

    impl $T {
      /// The exact value as a `BigDecimal`, for maths this crate doesn't
      /// provide.
      ///
      #[doc = concat!("```\nuse decimix::{", stringify!($T), ", Round, ", $mac, "};\n")]
      #[doc = concat!("let b = ", $mac, "!(113.725).to_big();")]
      #[doc = "assert_eq!(b.to_string(), \"113.7250000000000000000\");"]
      #[doc = "```"]
      #[must_use]
      pub fn to_big(self) -> BigDecimal {
        self.into()
      }

      /// Converts a `BigDecimal`, rounding to 19 places with `mode`.
      ///
      /// Fails if the value is out of range. Extreme exponents are handled
      /// cheaply: a tiny value like `1e-1000000` rounds to zero (or the
      /// smallest step, for directed modes) without building a
      /// million-digit number.
      pub fn from_big(x: &BigDecimal, mode: Round) -> Result<Self, OutOfRange> {
        let _ = (x, mode);
        todo!("phase 6: from_big")
      }
    }

    impl TryFrom<&BigDecimal> for $T {
      type Error = TryFromBigError;

      /// Converts exactly: fails rather than round.
      fn try_from(x: &BigDecimal) -> Result<Self, TryFromBigError> {
        let _ = x;
        todo!("phase 6: TryFrom<&BigDecimal>")
      }
    }
  };
}

impl_big!(Dec19, "dec");
impl_big!(UDec19, "udec");
