//! Exact conversions to and from `bigdecimal::BigDecimal` (feature
//! `bigdecimal`).
//!
//! `BigDecimal` is the slow path for maths this crate doesn't do (square roots,
//! high-precision analytics). Going to `BigDecimal` is always exact. Coming
//! back either names its rounding (`from_big`) or refuses to round
//! (`TryFrom`).

use bigdecimal::BigDecimal;
use bigdecimal::num_bigint::{BigInt, BigUint, Sign};
use bigdecimal::num_traits::Zero;

use crate::ascii::Dropped;
use crate::round::decide;
use crate::{Dec19, OutOfRange, Round, TryFromBigError, UDec19};

/// The value of `x` in 10⁻¹⁹ steps, as a sign and magnitude.
///
/// With a rounding mode, the result is rounded; without one, it is
/// truncated toward zero, and the flag says whether anything non-zero was
/// dropped. Fails if the magnitude doesn't fit in a u128 (the caller checks
/// its own type's range).
///
/// A `BigDecimal` is `digits × 10^-scale`, and the scale can be anything,
/// e.g. `1e-1000000` has scale 1,000,000. So the stored value
/// `digits × 10^(19 - scale)` is only computed when it's cheap: large
/// results fail at once, and tiny ones are recognised by digit count.
fn to_steps(
  x: &BigDecimal,
  mode: Option<Round>,
) -> Result<(bool, u128, bool), OutOfRange> {
  let (digits, scale) = x.as_bigint_and_scale();
  let negative = digits.sign() == Sign::Minus;
  let magnitude = digits.magnitude();
  if magnitude.is_zero() {
    return Ok((false, 0, false));
  }
  let ten = BigUint::from(10u8);
  // How many powers of ten to multiply by (positive) or divide by
  // (negative). i128 so that no scale can overflow the subtraction.
  let shift = 19 - scale as i128;

  if shift >= 0 {
    // Multiply up: exact. Any non-zero number times 10³⁹ or more is beyond
    // a u128 (whose maximum is about 3.4 × 10³⁸), so don't compute that.
    if shift > 38 {
      return Err(OutOfRange);
    }
    let stored = magnitude * ten.pow(shift as u32);
    let stored = u128::try_from(&stored).map_err(|_| OutOfRange)?;
    return Ok((negative, stored, false));
  }

  // Divide by 10^k, rounding. If the digits have fewer than k decimal
  // digits, the number is below 10^(k-1), less than a tenth of 10^k: the
  // quotient is 0 and the remainder is non-zero but below half. That avoids
  // building 10^k for absurd k.
  let k = -shift;
  let (q, dropped) = if (x.digits() as i128) < k {
    (0, Dropped::BelowHalf)
  } else {
    // Here k is at most the number of digits, so 10^k costs no more than
    // the input itself.
    let unit = ten.pow(k as u32);
    let q = magnitude / &unit;
    let r = magnitude % &unit;
    let q = u128::try_from(&q).map_err(|_| OutOfRange)?;
    let dropped = if r.is_zero() {
      Dropped::Nothing
    } else {
      match (r << 1u32).cmp(&unit) {
        core::cmp::Ordering::Less => Dropped::BelowHalf,
        core::cmp::Ordering::Equal => Dropped::Half,
        core::cmp::Ordering::Greater => Dropped::AboveHalf,
      }
    };
    (q, dropped)
  };

  let inexact = !matches!(dropped, Dropped::Nothing);
  let Some(mode) = mode else {
    return Ok((negative, q, inexact));
  };
  let away = decide(
    mode,
    negative,
    q & 1 == 1,
    matches!(dropped, Dropped::AboveHalf),
    matches!(dropped, Dropped::Half),
    inexact,
  );
  let q = q.checked_add(away as u128).ok_or(OutOfRange)?;
  Ok((negative, q, false))
}

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
        let (negative, magnitude, _) = to_steps(x, Some(mode))?;
        Self::from_parts(negative, magnitude).ok_or(OutOfRange)
      }
    }

    impl TryFrom<&BigDecimal> for $T {
      type Error = TryFromBigError;

      /// Converts exactly: fails rather than round.
      fn try_from(x: &BigDecimal) -> Result<Self, TryFromBigError> {
        // Range first, then exactness, as when parsing text.
        let (negative, magnitude, inexact) =
          to_steps(x, None).map_err(|_| TryFromBigError::OutOfRange)?;
        let v = Self::from_parts(negative, magnitude)
          .ok_or(TryFromBigError::OutOfRange)?;
        if inexact {
          return Err(TryFromBigError::Inexact);
        }
        Ok(v)
      }
    }
  };
}

impl_big!(Dec19, "dec");
impl_big!(UDec19, "udec");
