//! `UDec19`: unsigned, 19 decimal places.

use crate::common::impl_fixed19;
use crate::consts::ONE_RAW;
use crate::{Dec19, OutOfRange, Round};

/// An unsigned decimal number with exactly **19 decimal places**.
///
/// Like [`Dec19`], but can never be negative, and the type says so. For
/// quantities, sizes and other amounts that are negative only by mistake.
/// Stored as a `u128` count of 10⁻¹⁹ steps, which gives a range of 0 to about
/// 3.4 × 10¹⁹.
///
/// Subtraction panics if the result would be negative, like `u128`. Use
/// [`checked_sub`](Self::checked_sub), [`abs_diff`](Self::abs_diff) or
/// [`signed_sub`](Self::signed_sub) when a negative difference is expected.
///
/// ```
/// use decimix::{dec, udec};
///
/// let ordered = udec!(1000);
/// let filled = udec!(250.5);
/// assert_eq!(ordered - filled, udec!(749.5));
/// assert_eq!(filled.checked_sub(ordered), None);
/// assert_eq!(filled.signed_sub(ordered), dec!(-749.5));
/// ```
///
/// Formatting with a precision, as in `format!("{x:.2}")`, rounds half-even,
/// as std does for floats. To choose the rounding, use
/// [`write_ascii_dp`](Self::write_ascii_dp) or [`round_dp`](Self::round_dp).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[must_use]
#[repr(transparent)]
pub struct UDec19(u128);

impl_fixed19! {
  type = UDec19,
  raw = u128,
  int = u64,
  mac = "udec",
  max_ascii_len = 40,
  sub_overflow = "UDec19 subtraction would be negative",
}

impl UDec19 {
  /// Nothing to flip in `to_key_bytes`: unsigned big-endian already sorts.
  const KEY_FLIP: u128 = 0;

  const LITERAL_RANGE_ERROR: &'static str =
    "udec! literal out of range (0 to about 3.4e19)";

  #[inline]
  pub(crate) const fn to_parts(self) -> (bool, u128) {
    (false, self.0)
  }

  #[inline]
  pub(crate) const fn raw_from_parts(
    negative: bool,
    magnitude: u128,
  ) -> Option<u128> {
    if negative && magnitude != 0 {
      None
    } else {
      Some(magnitude)
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

  /// The distance between two values, whichever is larger.
  #[inline]
  pub const fn abs_diff(self, other: Self) -> Self {
    Self(self.0.abs_diff(other.0))
  }

  /// `self - other` as a signed [`Dec19`], which may be negative.
  ///
  /// # Panics
  ///
  /// If the difference is outside [`Dec19`]'s range (only possible for
  /// values above about 1.7 × 10¹⁹).
  #[inline]
  #[track_caller]
  pub fn signed_sub(self, other: Self) -> Dec19 {
    self
      .checked_signed_sub(other)
      .expect("UDec19::signed_sub result out of Dec19 range")
  }

  /// `self - other` as a signed [`Dec19`], or `None` if it's outside
  /// [`Dec19`]'s range.
  #[must_use]
  #[inline]
  pub fn checked_signed_sub(self, other: Self) -> Option<Dec19> {
    let negative = self.0 < other.0;
    Dec19::from_parts(negative, self.0.abs_diff(other.0))
  }

  /// Multiplication, rounded to 19 places with `mode`.
  ///
  /// # Panics
  ///
  /// On overflow. See [`checked_mul`](Self::checked_mul).
  #[track_caller]
  #[inline]
  pub fn mul(self, rhs: Self, mode: Round) -> Self {
    self
      .checked_mul(rhs, mode)
      .expect("UDec19 overflow in multiplication")
  }

  /// Multiplication, rounded to 19 places with `mode`, or `None` on
  /// overflow.
  #[must_use]
  #[inline]
  pub fn checked_mul(self, rhs: Self, mode: Round) -> Option<Self> {
    crate::kernel::mul::mul_parts(self.0, rhs.0, false, mode).map(Self)
  }

  /// Multiplication, rounded to 19 places with `mode`, clamped to
  /// [`MAX`](Self::MAX) instead of overflowing.
  #[inline]
  pub fn saturating_mul(self, rhs: Self, mode: Round) -> Self {
    self.checked_mul(rhs, mode).unwrap_or(Self::MAX)
  }

  /// Division, rounded to 19 places with `mode`.
  ///
  /// # Panics
  ///
  /// If `rhs` is zero or the result overflows.
  #[track_caller]
  #[inline]
  pub fn div(self, rhs: Self, mode: Round) -> Self {
    self
      .checked_div(rhs, mode)
      .expect("UDec19 division by zero or overflow")
  }

  /// Division, rounded to 19 places with `mode`, or `None` if `rhs` is zero
  /// or the result overflows.
  #[must_use]
  #[inline]
  pub fn checked_div(self, rhs: Self, mode: Round) -> Option<Self> {
    crate::kernel::div::div_parts(self.0, rhs.0, false, mode).map(Self)
  }
}

// ---- Conversions ------------------------------------------------------------

macro_rules! from_int {
  ($($t:ty),*) => {$(
    impl From<$t> for UDec19 {
      #[inline]
      fn from(n: $t) -> Self {
        // Every value of this type times 10¹⁹ fits in a u128.
        Self(n as u128 * ONE_RAW)
      }
    }
  )*};
}

from_int!(u8, u16, u32, u64);

macro_rules! try_from_int {
  ($($t:ty),*) => {$(
    impl TryFrom<$t> for UDec19 {
      type Error = OutOfRange;

      #[inline]
      fn try_from(n: $t) -> Result<Self, OutOfRange> {
        u128::try_from(n)
          .ok()
          .and_then(|n| n.checked_mul(ONE_RAW))
          .map(Self)
          .ok_or(OutOfRange)
      }
    }
  )*};
}

try_from_int!(i8, i16, i32, i64, i128, u128);

impl TryFrom<Dec19> for UDec19 {
  type Error = OutOfRange;

  /// Fails if the value is negative.
  #[inline]
  fn try_from(v: Dec19) -> Result<Self, OutOfRange> {
    u128::try_from(v.to_raw()).map(Self).map_err(|_| OutOfRange)
  }
}
