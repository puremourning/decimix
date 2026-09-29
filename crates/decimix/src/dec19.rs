//! `Dec19`: signed, 19 decimal places.

use crate::common::impl_fixed19;
use crate::consts::ONE_RAW;
use crate::{Fixed19, OutOfRange, Round, UDec19};

/// A signed decimal number with exactly **19 decimal places**.
///
/// Stored as an `i128` count of 10⁻¹⁹ steps: 113.725 is stored as
/// 1,137,250,000,000,000,000,000. That gives a range of about ±1.7 × 10¹⁹ whole
/// units (±17,014,118,346,046,923,173.1687303715884105727) with 38 significant
/// digits, and every value has exactly one representation, so `==`, `<` and
/// hashing are plain integer operations.
///
/// Operators exist only where the result is exact: `+`, `-`, unary `-`,
/// comparisons and `*` by an integer. They panic on overflow, in release
/// builds too; the `checked_*` and `saturating_*` methods don't. Anything that
/// can round is a method taking a [`Round`]:
///
/// ```
/// use decimix::{Round, dec};
///
/// let px = dec!(113.725);
/// let qty = dec!(250);
/// let spread = dec!(113.73) - px; // exact
/// assert_eq!(spread, dec!(0.005));
/// assert_eq!(px * 4, dec!(454.9)); // exact
/// let tick = dec!(0.01);
/// assert_eq!(px.round_to(tick, Round::HalfEven), dec!(113.72));
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[must_use]
#[repr(transparent)]
pub struct Dec19(i128);

impl_fixed19! {
  type = Dec19,
  raw = i128,
  int = i64,
  mac = "dec",
  max_ascii_len = 41,
  sub_overflow = "Dec19 overflow in subtraction",
}

impl Dec19 {
  const LITERAL_RANGE_ERROR: &'static str =
    "dec! literal out of range (about ±1.7e19)";

  #[inline]
  pub(crate) const fn to_parts(self) -> (bool, u128) {
    (self.0 < 0, self.0.unsigned_abs())
  }

  #[inline]
  pub(crate) const fn raw_from_parts(
    negative: bool,
    magnitude: u128,
  ) -> Option<i128> {
    if negative {
      // The most negative i128 is one further from zero than the most
      // positive, so a magnitude of exactly 2¹²⁷ is allowed here.
      if magnitude > 1u128 << 127 {
        None
      } else {
        Some((magnitude as i128).wrapping_neg())
      }
    } else if magnitude > i128::MAX as u128 {
      None
    } else {
      Some(magnitude as i128)
    }
  }

  #[inline]
  pub(crate) const fn from_parts(
    negative: bool,
    magnitude: u128,
  ) -> Option<Self> {
    match Self::raw_from_parts(negative, magnitude) {
      Some(raw) => Some(Self(raw)),
      None => None,
    }
  }

  // ---- Sign ----------------------------------------------------------------

  /// True if the value is below zero.
  #[must_use]
  #[inline]
  pub const fn is_negative(self) -> bool {
    self.0 < 0
  }

  /// True if the value is above zero.
  #[must_use]
  #[inline]
  pub const fn is_positive(self) -> bool {
    self.0 > 0
  }

  /// The absolute value.
  ///
  /// # Panics
  ///
  /// For [`MIN`](Self::MIN), whose absolute value doesn't fit.
  #[inline]
  #[track_caller]
  pub const fn abs(self) -> Self {
    match self.checked_abs() {
      Some(v) => v,
      None => panic!("Dec19 overflow in abs"),
    }
  }

  /// The absolute value, or `None` for [`MIN`](Self::MIN).
  #[must_use]
  #[inline]
  pub const fn checked_abs(self) -> Option<Self> {
    match self.0.checked_abs() {
      Some(v) => Some(Self(v)),
      None => None,
    }
  }

  /// The absolute value, with [`MIN`](Self::MIN) giving [`MAX`](Self::MAX).
  #[inline]
  pub const fn saturating_abs(self) -> Self {
    Self(self.0.saturating_abs())
  }

  /// Negation, or `None` for [`MIN`](Self::MIN).
  #[must_use]
  #[inline]
  pub const fn checked_neg(self) -> Option<Self> {
    match self.0.checked_neg() {
      Some(v) => Some(Self(v)),
      None => None,
    }
  }

  /// Negation, with [`MIN`](Self::MIN) giving [`MAX`](Self::MAX).
  #[inline]
  pub const fn saturating_neg(self) -> Self {
    Self(self.0.saturating_neg())
  }

  // ---- Multiplication and division that round ----------------------------

  /// Multiplication, rounded to 19 places with `mode`.
  ///
  /// `rhs` may be a [`Dec19`] or a [`UDec19`].
  ///
  /// ```
  /// use decimix::{Round, dec};
  ///
  /// let notional = dec!(113.725).mul(dec!(1234.5678901), Round::HalfEven);
  /// assert_eq!(notional, dec!(140401.2333016225)); // exact here
  /// ```
  ///
  /// # Panics
  ///
  /// On overflow. See [`checked_mul`](Self::checked_mul).
  #[track_caller]
  #[inline]
  pub fn mul<R: Fixed19>(self, rhs: R, mode: Round) -> Self {
    self
      .checked_mul(rhs, mode)
      .expect("Dec19 overflow in multiplication")
  }

  /// Multiplication, rounded to 19 places with `mode`, or `None` on
  /// overflow.
  #[must_use]
  #[inline]
  pub fn checked_mul<R: Fixed19>(self, rhs: R, mode: Round) -> Option<Self> {
    // Work on sizes and put the sign back at the end: negative exactly when
    // one input is negative.
    let (neg_a, a) = self.to_parts();
    let (neg_b, b) = rhs.__to_parts();
    let negative = neg_a != neg_b;
    let m = crate::kernel::mul::mul_parts(a, b, negative, mode)?;
    Self::from_parts(negative, m)
  }

  /// Multiplication, rounded to 19 places with `mode`, clamped to
  /// [`MIN`](Self::MIN)..=[`MAX`](Self::MAX) instead of overflowing.
  #[inline]
  pub fn saturating_mul<R: Fixed19>(self, rhs: R, mode: Round) -> Self {
    match self.checked_mul(rhs, mode) {
      Some(v) => v,
      // Overflow: clamp toward the sign the product would have had.
      None if self.is_negative() != rhs.__to_parts().0 => Self::MIN,
      None => Self::MAX,
    }
  }

  /// Division, rounded to 19 places with `mode`.
  ///
  /// `rhs` may be a [`Dec19`] or a [`UDec19`]. For dividing by a whole
  /// number, [`div_int`](Self::div_int) is faster; for lot counts, use the
  /// exact [`div_floor`](Self::div_floor).
  ///
  /// # Panics
  ///
  /// If `rhs` is zero or the result overflows.
  #[track_caller]
  #[inline]
  pub fn div<R: Fixed19>(self, rhs: R, mode: Round) -> Self {
    self
      .checked_div(rhs, mode)
      .expect("Dec19 division by zero or overflow")
  }

  /// Division, rounded to 19 places with `mode`, or `None` if `rhs` is zero
  /// or the result overflows.
  #[must_use]
  #[inline]
  pub fn checked_div<R: Fixed19>(self, rhs: R, mode: Round) -> Option<Self> {
    let (neg_a, a) = self.to_parts();
    let (neg_b, b) = rhs.__to_parts();
    let negative = neg_a != neg_b;
    let m = crate::kernel::div::div_parts(a, b, negative, mode)?;
    Self::from_parts(negative, m)
  }
}

impl core::ops::Neg for Dec19 {
  type Output = Self;

  #[inline]
  #[track_caller]
  fn neg(self) -> Self {
    self.checked_neg().expect("Dec19 overflow in negation")
  }
}

// ---- Conversions ------------------------------------------------------------

macro_rules! from_int {
  ($($t:ty),*) => {$(
    impl From<$t> for Dec19 {
      #[inline]
      fn from(n: $t) -> Self {
        // Every value of this type times 10¹⁹ fits in an i128.
        Self(n as i128 * ONE_RAW as i128)
      }
    }
  )*};
}

from_int!(i8, i16, i32, i64, u8, u16, u32);

macro_rules! try_from_int {
  ($($t:ty),*) => {$(
    impl TryFrom<$t> for Dec19 {
      type Error = OutOfRange;

      #[inline]
      fn try_from(n: $t) -> Result<Self, OutOfRange> {
        i128::try_from(n)
          .ok()
          .and_then(|n| n.checked_mul(ONE_RAW as i128))
          .map(Self)
          .ok_or(OutOfRange)
      }
    }
  )*};
}

try_from_int!(u64, i128, u128);

impl TryFrom<UDec19> for Dec19 {
  type Error = OutOfRange;

  /// Fails only above [`Dec19::MAX`], about 1.7 × 10¹⁹.
  #[inline]
  fn try_from(v: UDec19) -> Result<Self, OutOfRange> {
    i128::try_from(v.to_raw()).map(Self).map_err(|_| OutOfRange)
  }
}
