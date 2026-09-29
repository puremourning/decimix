//! The API shared by `Dec19` and `UDec19`, generated for each by one macro
//! (the way std generates the methods of its integer types).
//!
//! Each type provides three private `const fn`s that everything here builds
//! on, so the shared code never has to care about the sign:
//! - `to_parts(self) -> (bool, u128)`: sign and magnitude;
//! - `raw_from_parts(bool, u128) -> Option<raw>`: back again, `None` if out of
//!   range;
//! - `from_parts(bool, u128) -> Option<Self>`: the same, wrapped.

/// Sign and magnitude of the plain integers used by `div_int` and
/// `mul_int`.
pub(crate) trait IntParts: Copy {
  fn parts(self) -> (bool, u128);
}

impl IntParts for i64 {
  fn parts(self) -> (bool, u128) {
    (self < 0, self.unsigned_abs() as u128)
  }
}

impl IntParts for u64 {
  fn parts(self) -> (bool, u128) {
    (false, self as u128)
  }
}

/// Divides a magnitude, rounding the quotient with `mode`. `negative` is the
/// sign of the final result.
#[inline]
pub(crate) fn div_round(
  magnitude: u128,
  divisor: u128,
  negative: bool,
  mode: crate::Round,
) -> u128 {
  let q = magnitude / divisor;
  let r = magnitude % divisor;
  // q + 1 cannot overflow: if the divisor is 1 the remainder is 0 and
  // round_away says no.
  q + crate::round::round_away(mode, negative, q & 1 == 1, r, divisor) as u128
}

macro_rules! impl_fixed19 {
  (
    type = $T:ident,
    raw = $raw:ty,
    int = $int:ty,
    mac = $mac:literal,
    max_ascii_len = $max_ascii_len:expr,
    sub_overflow = $sub_overflow:literal,
  ) => {
    impl $T {
      /// Zero.
      pub const ZERO: Self = Self(0);
      /// One.
      pub const ONE: Self = Self($crate::consts::ONE_RAW as $raw);
      /// The largest value.
      pub const MAX: Self = Self(<$raw>::MAX);
      /// The smallest (most negative) value.
      pub const MIN: Self = Self(<$raw>::MIN);
      /// The smallest step between two values: 0.0000000000000000001
      /// (10⁻¹⁹).
      ///
      /// Every value is a whole number of these steps.
      #[doc(alias = "ULP", alias = "resolution")]
      pub const SMALLEST_STEP: Self = Self(1);
      /// Number of decimal places: always 19.
      ///
      /// Every value has exactly 19 digits after the decimal point (trailing
      /// zeros are simply not printed). This is *not* SQL `DECIMAL(19)`,
      /// which means 19 digits in total.
      #[doc(alias = "scale")]
      pub const DECIMAL_PLACES: u32 = 19;
      /// The longest text `write_ascii` can produce, in bytes. A buffer this
      /// long always fits the shortest form.
      pub const MAX_ASCII_LEN: usize = $max_ascii_len;

      // ---- Raw representation -------------------------------------------

      /// Builds a value from its stored integer: the number of 10⁻¹⁹ steps.
      ///
      /// For storage and wire formats. `from_raw(1)` is
      /// [`SMALLEST_STEP`](Self::SMALLEST_STEP), and `from_raw(10¹⁹)` is 1.
      #[inline]
      pub const fn from_raw(raw: $raw) -> Self {
        Self(raw)
      }

      /// The stored integer: the value in 10⁻¹⁹ steps.
      ///
      /// For storage and wire formats. This is **not** the whole-number part:
      /// 5 gives 50,000,000,000,000,000,000. Use
      /// [`to_int`](Self::to_int) for that.
      #[must_use]
      #[inline]
      pub const fn to_raw(self) -> $raw {
        self.0
      }

      /// The stored integer as little-endian bytes.
      #[must_use]
      #[inline]
      pub const fn to_le_bytes(self) -> [u8; 16] {
        self.0.to_le_bytes()
      }

      /// Builds a value from the little-endian bytes of its stored integer.
      #[inline]
      pub const fn from_le_bytes(bytes: [u8; 16]) -> Self {
        Self(<$raw>::from_le_bytes(bytes))
      }

      /// The stored integer as big-endian bytes.
      #[must_use]
      #[inline]
      pub const fn to_be_bytes(self) -> [u8; 16] {
        self.0.to_be_bytes()
      }

      /// Builds a value from the big-endian bytes of its stored integer.
      #[inline]
      pub const fn from_be_bytes(bytes: [u8; 16]) -> Self {
        Self(<$raw>::from_be_bytes(bytes))
      }

      /// Builds a value from an integer with an implied number of decimal
      /// places, as used by binary exchange protocols and the old
      /// `(mantissa, scale)` pairs.
      ///
      /// `from_scaled(1_137_250, 4)` is 113.725. Always exact. Returns `None`
      /// if `decimal_places` is more than 19.
      #[must_use]
      #[inline]
      pub const fn from_scaled(value: $int, decimal_places: u32) -> Option<Self> {
        if decimal_places > 19 {
          return None;
        }
        // Cannot overflow: the largest 64-bit integer times 10¹⁹ is well
        // inside the 128-bit range.
        Some(Self(
          value as $raw * $crate::consts::POW10[19 - decimal_places as usize] as $raw,
        ))
      }

      /// The value as an integer with an implied number of decimal places,
      /// rounded with `mode`: the reverse of
      /// [`from_scaled`](Self::from_scaled).
      ///
      /// `113.725` at 4 places is `1_137_250`. Fails if the result does not
      /// fit the integer type.
      pub fn to_scaled(
        self,
        decimal_places: u32,
        mode: $crate::Round,
      ) -> Result<$int, $crate::OutOfRange> {
        let (negative, magnitude) = self.to_parts();
        let scaled = if decimal_places <= 19 {
          $crate::common::div_round(
            magnitude,
            $crate::consts::POW10[19 - decimal_places as usize],
            negative,
            mode,
          )
        } else if magnitude == 0 {
          // Zero is zero at any number of places (and 10^k for large k
          // doesn't fit in a u128).
          0
        } else {
          // More places than stored: multiply up, exactly.
          let factor = *$crate::consts::POW10
            .get(decimal_places as usize - 19)
            .ok_or($crate::OutOfRange)?;
          magnitude.checked_mul(factor).ok_or($crate::OutOfRange)?
        };
        let raw =
          Self::raw_from_parts(negative, scaled).ok_or($crate::OutOfRange)?;
        <$int>::try_from(raw).map_err(|_| $crate::OutOfRange)
      }

      // ---- Tests ----------------------------------------------------------

      /// True if the value is zero.
      #[must_use]
      #[inline]
      pub const fn is_zero(self) -> bool {
        self.0 == 0
      }

      /// True if the value is a whole number (no non-zero decimal places).
      #[must_use]
      #[inline]
      pub const fn is_integer(self) -> bool {
        self.0 % ($crate::consts::ONE_RAW as $raw) == 0
      }

      // ---- Exact arithmetic ---------------------------------------------

      /// Addition, or `None` on overflow.
      #[must_use]
      #[inline]
      pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        match self.0.checked_add(rhs.0) {
          Some(v) => Some(Self(v)),
          None => None,
        }
      }

      /// Subtraction, or `None` on overflow.
      #[must_use]
      #[inline]
      pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        match self.0.checked_sub(rhs.0) {
          Some(v) => Some(Self(v)),
          None => None,
        }
      }

      /// Addition, clamped to [`MIN`](Self::MIN)..=[`MAX`](Self::MAX)
      /// instead of overflowing.
      #[inline]
      pub const fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
      }

      /// Subtraction, clamped to [`MIN`](Self::MIN)..=[`MAX`](Self::MAX)
      /// instead of overflowing.
      #[inline]
      pub const fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
      }

      /// Multiplication by an integer, or `None` on overflow. Exact.
      #[must_use]
      #[inline]
      pub const fn checked_mul_int(self, n: $int) -> Option<Self> {
        match self.0.checked_mul(n as $raw) {
          Some(v) => Some(Self(v)),
          None => None,
        }
      }

      /// Multiplication by an integer, clamped to
      /// [`MIN`](Self::MIN)..=[`MAX`](Self::MAX) instead of overflowing.
      #[inline]
      pub const fn saturating_mul_int(self, n: $int) -> Self {
        Self(self.0.saturating_mul(n as $raw))
      }

      // ---- Division that doesn't round ------------------------------------

      /// How many whole `rhs` fit in `self`, rounding toward negative
      /// infinity. Exact.
      ///
      /// For lot and clip counts. Pairs with
      /// [`rem_euclid`](Self::rem_euclid):
      /// `self == rhs * self.div_floor(rhs) + self.rem_euclid(rhs)`.
      ///
      /// # Panics
      ///
      /// If `rhs` is zero.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const fn div_floor(self, rhs: Self) -> $raw {
        self.0.div_euclid(rhs.0)
      }

      /// What is left after taking whole `rhs` out of `self`. Exact, and
      /// never negative.
      ///
      /// # Panics
      ///
      /// If `rhs` is zero.
      #[inline]
      #[track_caller]
      pub const fn rem_euclid(self, rhs: Self) -> Self {
        Self(self.0.rem_euclid(rhs.0))
      }

      // ---- Rounding --------------------------------------------------------

      /// Rounds to a multiple of `step` (a tick or lot size), using `mode`.
      ///
      /// Correct for negative values: `Floor` always moves toward negative
      /// infinity.
      ///
      #[doc = concat!("```\nuse decimix::{", stringify!($T), ", Round, ", $mac, "};\n")]
      #[doc = concat!("let tick = ", $mac, "!(0.005);")]
      #[doc = concat!("assert_eq!(", $mac, "!(113.7268).round_to(tick, Round::Floor), ", $mac, "!(113.725));")]
      #[doc = concat!("assert_eq!(", $mac, "!(113.7268).round_to(tick, Round::HalfEven), ", $mac, "!(113.725));")]
      #[doc = concat!("assert_eq!(", $mac, "!(113.7276).round_to(tick, Round::HalfEven), ", $mac, "!(113.73));")]
      #[doc = "```"]
      ///
      /// # Panics
      ///
      /// If `step` is not positive, or the result overflows.
      #[track_caller]
      pub fn round_to(self, step: Self, mode: $crate::Round) -> Self {
        assert!(step.0 > 0, "round_to: step must be positive");
        let (negative, magnitude) = self.to_parts();
        let step = step.0 as u128;
        // Count whole steps, rounding the count; then multiply back.
        let steps = $crate::common::div_round(magnitude, step, negative, mode);
        steps
          .checked_mul(step)
          .and_then(|m| Self::from_parts(negative, m))
          .expect(concat!(stringify!($T), " overflow in round_to"))
      }

      /// Rounds to `places` decimal places, using `mode`.
      ///
      /// `round_dp(2, Round::HalfEven)` rounds to cents. With 19 or more
      /// places there is nothing to round.
      ///
      /// # Panics
      ///
      /// If the result overflows (only possible when rounding away from zero
      /// right next to [`MAX`](Self::MAX) or [`MIN`](Self::MIN)).
      #[track_caller]
      pub fn round_dp(self, places: u32, mode: $crate::Round) -> Self {
        if places >= 19 {
          return self;
        }
        self.round_to(
          Self($crate::consts::POW10[19 - places as usize] as $raw),
          mode,
        )
      }

      /// The whole-number value, rounded with `mode`.
      ///
      /// `2.5.to_int(Round::HalfEven)` is 2; `2.5.to_int(Round::Ceiling)`
      /// is 3. Always fits: the whole part of a value is at most about
      /// 3.4 × 10¹⁹.
      #[must_use]
      pub fn to_int(self, mode: $crate::Round) -> $raw {
        let (negative, magnitude) = self.to_parts();
        let q = $crate::common::div_round(
          magnitude,
          $crate::consts::ONE_RAW,
          negative,
          mode,
        );
        Self::raw_from_parts(negative, q).expect("whole part always fits")
      }

      // ---- Division that rounds ------------------------------------------

      /// Division by an integer, rounded with `mode`.
      ///
      /// For averages: `total.div_int(count, Round::HalfEven)`.
      ///
      /// # Panics
      ///
      /// If `n` is zero or the result overflows.
      #[track_caller]
      pub fn div_int(self, n: $int, mode: $crate::Round) -> Self {
        assert!(n != 0, "div_int: division by zero");
        self
          .checked_div_int(n, mode)
          .expect(concat!(stringify!($T), " overflow in div_int"))
      }

      /// Division by an integer, rounded with `mode`, or `None` if `n` is
      /// zero or the result overflows.
      #[must_use]
      pub fn checked_div_int(self, n: $int, mode: $crate::Round) -> Option<Self> {
        let (neg_a, a) = self.to_parts();
        let (neg_n, n) = $crate::common::IntParts::parts(n);
        if n == 0 {
          return None;
        }
        let negative = neg_a != neg_n;
        Self::from_parts(negative, $crate::common::div_round(a, n, negative, mode))
      }

      // ---- Text --------------------------------------------------------

      /// Reads ASCII decimal text such as `b"113.725"` or `b"-0.5"`,
      /// exactly.
      ///
      /// Accepts an optional sign, digits and an optional decimal point. No
      /// exponents, spaces, thousands separators or locales. More than 19
      /// decimal places is an error unless the extra digits are zeros; use
      /// [`from_ascii_round`](Self::from_ascii_round) to round instead.
      /// Works directly on received message bytes: no UTF-8 check.
      pub const fn from_ascii(bytes: &[u8]) -> Result<Self, $crate::ParseError> {
        match $crate::ascii::parse(bytes, 0, bytes.len(), None) {
          Ok(p) => Self::from_parsed(p),
          Err(e) => Err(e),
        }
      }

      /// Reads ASCII decimal text like [`from_ascii`](Self::from_ascii), but
      /// rounds digits beyond the 19th decimal place with `mode`.
      pub const fn from_ascii_round(
        bytes: &[u8],
        mode: $crate::Round,
      ) -> Result<Self, $crate::ParseError> {
        match $crate::ascii::parse(bytes, 0, bytes.len(), Some(mode)) {
          Ok(p) => Self::from_parsed(p),
          Err(e) => Err(e),
        }
      }

      /// Reads decimal text like `str::parse`, but rounds digits beyond the
      /// 19th decimal place with `mode`.
      pub const fn parse_round(
        s: &str,
        mode: $crate::Round,
      ) -> Result<Self, $crate::ParseError> {
        Self::from_ascii_round(s.as_bytes(), mode)
      }

      /// Writes the shortest exact text (`113.725`, `-0.5`, `250`) into
      /// `out` and returns the number of bytes written.
      ///
      /// For writing straight into an outgoing message buffer.
      /// [`MAX_ASCII_LEN`](Self::MAX_ASCII_LEN) bytes are always enough.
      pub fn write_ascii(self, out: &mut [u8]) -> Result<usize, $crate::BufferTooSmall> {
        let (negative, magnitude) = self.to_parts();
        $crate::ascii::write_shortest(negative, magnitude, out)
      }

      /// Writes exactly `places` decimal places into `out`, rounding with
      /// `mode` if needed (`113.725` at 4 places is `113.7250`, at 2 places
      /// half-even is `113.72`). Returns the number of bytes written.
      ///
      /// For venues that require a fixed number of decimals.
      pub fn write_ascii_dp(
        self,
        out: &mut [u8],
        places: u32,
        mode: $crate::Round,
      ) -> Result<usize, $crate::BufferTooSmall> {
        let (negative, magnitude) = self.to_parts();
        $crate::ascii::write_places(negative, magnitude, places, mode, out)
      }

      /// The shortest exact text, in a small stack buffer.
      #[must_use]
      pub fn to_ascii(self) -> $crate::AsciiBuf {
        let (negative, magnitude) = self.to_parts();
        $crate::AsciiBuf::new(negative, magnitude)
      }

      // ---- f64 boundary -------------------------------------------------

      /// **Not for arithmetic.** Converts to the nearest `f64`, for output to
      /// systems and APIs that use doubles.
      ///
      /// The result is exactly what parsing this number's text as a double
      /// gives (`"113.725".parse::<f64>()`), so a peer that reads our text
      /// and one that reads this double agree bit for bit.
      ///
      /// Doubles can't hold most decimals exactly (0.1 has no exact binary
      /// form, just as 1/3 has no exact decimal one), and every calculation
      /// on them can add error. If you need arithmetic this type doesn't
      /// provide, convert with `to_big()` (feature `bigdecimal`) instead,
      /// which is exact.
      ///
      /// Core crates can ban this method with Clippy's `disallowed-methods`;
      /// see the crate documentation.
      #[doc(alias = "to_f64", alias = "as_f64", alias = "into_f64")]
      #[must_use]
      pub fn to_f64_lossy(self) -> f64 {
        todo!("phase 6: to_f64_lossy")
      }

      /// **Not for arithmetic.** Converts a double from a system or API that
      /// uses them, keeping only the precision you trust.
      ///
      /// Two steps:
      /// 1. Take the shortest decimal that converts back to exactly this
      ///    double. That is what every language prints for it: `0.1` for the
      ///    double nearest 0.1, not its exact binary value
      ///    `0.1000000000000000055…`.
      /// 2. Round that to the nearest multiple of `step`, using `mode`.
      ///
      /// `step` is the finest precision you actually trust in the incoming
      /// double: e.g. `0.000001`, or the instrument's tick size. A price
      /// computed upstream as `0.1 + 0.2` arrives as `0.30000000000000004`;
      /// with a step of `0.000001` it becomes exactly `0.3`. Pass
      /// [`SMALLEST_STEP`](Self::SMALLEST_STEP) to keep every digit.
      ///
      /// `-0.0` becomes zero. Fails on NaN, infinity or out-of-range values.
      ///
      /// # Panics
      ///
      /// If `step` is not positive.
      #[doc(alias = "from_f64")]
      pub fn from_f64_lossy(
        x: f64,
        step: Self,
        mode: $crate::Round,
      ) -> Result<Self, $crate::FromF64Error> {
        let _ = (x, step, mode);
        todo!("phase 6: from_f64_lossy")
      }

      // ---- Macro support -------------------------------------------------

      #[doc(hidden)]
      pub const fn __from_literal(s: &str) -> Result<Self, &'static str> {
        match $crate::ascii::parse_literal(s) {
          Ok(p) => match Self::from_parsed(p) {
            Ok(v) => Ok(v),
            Err($crate::ParseError::OutOfRange) => Err(Self::LITERAL_RANGE_ERROR),
            Err(e) => Err(e.as_str()),
          },
          Err(e) => Err(e),
        }
      }

      /// Range check first, then exactness: see `ascii::Parsed`.
      const fn from_parsed(
        p: $crate::ascii::Parsed,
      ) -> Result<Self, $crate::ParseError> {
        match Self::from_parts(p.negative, p.magnitude) {
          None => Err($crate::ParseError::OutOfRange),
          Some(_) if p.inexact => Err($crate::ParseError::TooPrecise),
          Some(v) => Ok(v),
        }
      }
    }

    impl $crate::sealed::Sealed for $T {}

    impl $crate::Fixed19 for $T {
      #[inline]
      fn __to_parts(self) -> (bool, u128) {
        self.to_parts()
      }
    }

    // ---- Operators ----------------------------------------------------------

    impl ::core::ops::Add for $T {
      type Output = Self;

      #[inline]
      #[track_caller]
      fn add(self, rhs: Self) -> Self {
        self
          .checked_add(rhs)
          .expect(concat!(stringify!($T), " overflow in addition"))
      }
    }

    impl ::core::ops::Sub for $T {
      type Output = Self;

      #[inline]
      #[track_caller]
      fn sub(self, rhs: Self) -> Self {
        self
          .checked_sub(rhs)
          .expect($sub_overflow)
      }
    }

    impl ::core::ops::AddAssign for $T {
      #[inline]
      #[track_caller]
      fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
      }
    }

    impl ::core::ops::SubAssign for $T {
      #[inline]
      #[track_caller]
      fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
      }
    }

    impl ::core::ops::Mul<$int> for $T {
      type Output = Self;

      #[inline]
      #[track_caller]
      fn mul(self, n: $int) -> Self {
        self
          .checked_mul_int(n)
          .expect(concat!(stringify!($T), " overflow in multiplication"))
      }
    }

    impl ::core::ops::Mul<$T> for $int {
      type Output = $T;

      #[inline]
      #[track_caller]
      fn mul(self, rhs: $T) -> $T {
        rhs * self
      }
    }

    impl ::core::ops::MulAssign<$int> for $T {
      #[inline]
      #[track_caller]
      fn mul_assign(&mut self, n: $int) {
        *self = *self * n;
      }
    }

    impl ::core::iter::Sum for $T {
      #[track_caller]
      fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |a, b| a + b)
      }
    }

    impl<'a> ::core::iter::Sum<&'a $T> for $T {
      #[track_caller]
      fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |a, b| a + *b)
      }
    }

    // ---- Text ---------------------------------------------------------------

    impl ::core::fmt::Display for $T {
      fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        let (negative, magnitude) = self.to_parts();
        $crate::ascii::display(negative, magnitude, f)
      }
    }

    impl ::core::fmt::Debug for $T {
      fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        write!(f, concat!(stringify!($T), "({})"), self)
      }
    }

    impl ::core::str::FromStr for $T {
      type Err = $crate::ParseError;

      fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_ascii(s.as_bytes())
      }
    }
  };
}

pub(crate) use impl_fixed19;
