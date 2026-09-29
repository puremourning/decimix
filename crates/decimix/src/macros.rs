//! `dec!`, `udec!` and `newtype!`.

/// A [`Dec19`](crate::Dec19) literal, checked at compile time.
///
/// Takes a number literal or a string literal. The literal's source text is
/// read as decimal digits, so it never passes through `f64`: `dec!(0.1)` is
/// exactly one tenth.
///
/// ```
/// use decimix::{Dec19, dec};
///
/// const TICK: Dec19 = dec!(0.005);
/// let px = dec!(113.725);
/// assert_eq!(px, dec!("113.725"));
/// assert_eq!(dec!(-0.25), -dec!(0.25));
/// assert_eq!(dec!(1_000_000), Dec19::from(1_000_000));
/// ```
///
/// A bad literal is a compile error: more than 19 decimal places, out of
/// range, an exponent (`1e-5`) or a suffix (`1.5f64`).
///
/// ```compile_fail
/// let x = decimix::dec!(1.5f64);
/// ```
///
/// ```compile_fail
/// let x = decimix::dec!(1e-5);
/// ```
///
/// ```compile_fail
/// let x = decimix::dec!(0.12345678901234567891); // 20 places
/// ```
///
/// A string literal reads exactly like [`FromStr`](core::str::FromStr), so
/// underscores are only for number literals:
///
/// ```compile_fail
/// let x = decimix::dec!("1_000");
/// ```
#[macro_export]
macro_rules! dec {
  ($lit:literal) => {
    const {
      match $crate::Dec19::__from_literal(::core::stringify!($lit)) {
        ::core::result::Result::Ok(v) => v,
        ::core::result::Result::Err(msg) => ::core::panic!("{}", msg),
      }
    }
  };
}

/// A [`UDec19`](crate::UDec19) literal, checked at compile time.
///
/// Like [`dec!`], and a negative literal is also a compile error.
///
/// ```
/// use decimix::udec;
///
/// let qty = udec!(1250.5);
/// assert_eq!(qty.to_string(), "1250.5");
/// ```
///
/// ```compile_fail
/// let x = decimix::udec!(-1);
/// ```
#[macro_export]
macro_rules! udec {
  ($lit:literal) => {
    const {
      match $crate::UDec19::__from_literal(::core::stringify!($lit)) {
        ::core::result::Result::Ok(v) => v,
        ::core::result::Result::Err(msg) => ::core::panic!("{}", msg),
      }
    }
  };
}

/// Defines a domain type (a price, a quantity, a rate…) wrapping
/// [`Dec19`](crate::Dec19) or [`UDec19`](crate::UDec19).
///
/// Distinct domain types can't be mixed by accident: with `Price` and `Qty`
/// defined this way, `price + qty` doesn't compile. Products between
/// different domain types (price × quantity → notional) are left for you to
/// define, since only you know which ones make sense.
///
/// ```
/// use decimix::{Dec19, Round, dec, newtype};
///
/// newtype! {
///   /// Commission rate.
///   pub struct FeeRate(Dec19);
/// }
///
/// let r = FeeRate::new(dec!(0.0002));
/// assert_eq!(r + r, FeeRate::new(dec!(0.0004)));
/// assert_eq!(r.get(), dec!(0.0002));
/// assert_eq!(format!("{r:?}"), "FeeRate(0.0002)");
/// ```
///
/// The base type must be written exactly `Dec19` or `UDec19`. The generated
/// type is `#[repr(transparent)]` with a private field, and gets:
/// - `new`/`get` (both `const`), `ZERO`, and `From<Type> for Dec19`;
/// - `Copy`, `Eq`, `Ord`, `Hash`, `Default`;
/// - same-type `+`, `-` (and unary `-` for `Dec19`), `+=`, `-=`, `Sum`, and
///   `*` by an integer (`i64`, or `u64` for `UDec19`), with `checked_*` and
///   `saturating_*` versions;
/// - `round_to`, `round_dp`, `div_euclid`, `rem_euclid`, `div_int`, and
///   `mul_dec` (multiply by a plain base-type value, e.g. a ratio);
/// - `Display`, `Debug` (`FeeRate(0.0002)`), `FromStr`, `from_ascii`,
///   `write_ascii` and `to_ascii`.
///
/// It deliberately has **no** `f64` methods: use `.get().to_f64_lossy()`,
/// so the lint rules only need to name the base types.
#[macro_export]
macro_rules! newtype {
  (
    $(#[$meta:meta])*
    $vis:vis struct $name:ident(Dec19);
  ) => {
    $crate::__newtype_common! {
      $(#[$meta])* $vis $name, $crate::Dec19, i128, i64
    }

    impl $name {
      /// True if the value is below zero.
      #[must_use]
      #[inline]
      pub const fn is_negative(self) -> bool {
        self.0.is_negative()
      }

      /// The absolute value.
      ///
      /// # Panics
      ///
      /// For the most negative value, whose absolute value doesn't fit.
      #[inline]
      #[track_caller]
      pub const fn abs(self) -> Self {
        Self(self.0.abs())
      }

      /// Negation, or `None` on overflow.
      #[must_use]
      #[inline]
      pub const fn checked_neg(self) -> ::core::option::Option<Self> {
        match self.0.checked_neg() {
          ::core::option::Option::Some(v) => ::core::option::Option::Some(Self(v)),
          ::core::option::Option::None => ::core::option::Option::None,
        }
      }
    }

    impl ::core::ops::Neg for $name {
      type Output = Self;

      #[inline]
      #[track_caller]
      fn neg(self) -> Self {
        Self(-self.0)
      }
    }
  };

  (
    $(#[$meta:meta])*
    $vis:vis struct $name:ident(UDec19);
  ) => {
    $crate::__newtype_common! {
      $(#[$meta])* $vis $name, $crate::UDec19, u128, u64
    }
  };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __newtype_common {
  (
    $(#[$meta:meta])* $vis:vis $name:ident, $base:ty, $raw:ty, $int:ty
  ) => {
    $(#[$meta])*
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    #[must_use]
    #[repr(transparent)]
    $vis struct $name($base);

    impl $name {
      /// Zero.
      pub const ZERO: Self = Self(<$base>::ZERO);

      /// Wraps a plain value.
      #[inline]
      pub const fn new(value: $base) -> Self {
        Self(value)
      }

      /// The plain value.
      #[inline]
      pub const fn get(self) -> $base {
        self.0
      }

      /// True if the value is zero.
      #[must_use]
      #[inline]
      pub const fn is_zero(self) -> bool {
        self.0.is_zero()
      }

      /// Addition, or `None` on overflow.
      #[must_use]
      #[inline]
      pub const fn checked_add(self, rhs: Self) -> ::core::option::Option<Self> {
        match self.0.checked_add(rhs.0) {
          ::core::option::Option::Some(v) => ::core::option::Option::Some(Self(v)),
          ::core::option::Option::None => ::core::option::Option::None,
        }
      }

      /// Subtraction, or `None` on overflow.
      #[must_use]
      #[inline]
      pub const fn checked_sub(self, rhs: Self) -> ::core::option::Option<Self> {
        match self.0.checked_sub(rhs.0) {
          ::core::option::Option::Some(v) => ::core::option::Option::Some(Self(v)),
          ::core::option::Option::None => ::core::option::Option::None,
        }
      }

      /// Addition, clamped to the range instead of overflowing.
      #[inline]
      pub const fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
      }

      /// Subtraction, clamped to the range instead of overflowing.
      #[inline]
      pub const fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
      }

      /// Multiplication by an integer, or `None` on overflow. Exact.
      #[must_use]
      #[inline]
      pub const fn checked_mul_int(self, n: $int) -> ::core::option::Option<Self> {
        match self.0.checked_mul_int(n) {
          ::core::option::Option::Some(v) => ::core::option::Option::Some(Self(v)),
          ::core::option::Option::None => ::core::option::Option::None,
        }
      }

      /// Multiplication by an integer, clamped to the range instead of
      /// overflowing.
      #[inline]
      pub const fn saturating_mul_int(self, n: $int) -> Self {
        Self(self.0.saturating_mul_int(n))
      }

      /// Rounds to a multiple of `step` (e.g. a tick size), using `mode`.
      #[inline]
      #[track_caller]
      pub fn round_to(self, step: Self, mode: $crate::Round) -> Self {
        Self(self.0.round_to(step.0, mode))
      }

      /// Rounds to `places` decimal places, using `mode`.
      #[inline]
      #[track_caller]
      pub fn round_dp(self, places: u32, mode: $crate::Round) -> Self {
        Self(self.0.round_dp(places, mode))
      }

      /// How many whole `rhs` fit in `self` (Euclidean division). Exact.
      /// See the base type's `div_euclid`.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const fn div_euclid(self, rhs: Self) -> $raw {
        self.0.div_euclid(rhs.0)
      }

      /// What is left after taking whole `rhs` out of `self`. Never
      /// negative. See the base type's `rem_euclid`.
      #[inline]
      #[track_caller]
      pub const fn rem_euclid(self, rhs: Self) -> Self {
        Self(self.0.rem_euclid(rhs.0))
      }

      /// Division by an integer, rounded with `mode`.
      #[inline]
      #[track_caller]
      pub fn div_int(self, n: $int, mode: $crate::Round) -> Self {
        Self(self.0.div_int(n, mode))
      }

      /// Division by an integer, rounded with `mode`, or `None` if `n` is
      /// zero or the result overflows.
      #[must_use]
      #[inline]
      pub fn checked_div_int(
        self,
        n: $int,
        mode: $crate::Round,
      ) -> ::core::option::Option<Self> {
        self.0.checked_div_int(n, mode).map(Self)
      }

      /// Multiplication by a plain value (a ratio, a factor), rounded with
      /// `mode`.
      #[inline]
      #[track_caller]
      pub fn mul_dec(self, factor: $base, mode: $crate::Round) -> Self {
        Self(self.0.mul(factor, mode))
      }

      /// Multiplication by a plain value, rounded with `mode`, or `None` on
      /// overflow.
      #[must_use]
      #[inline]
      pub fn checked_mul_dec(
        self,
        factor: $base,
        mode: $crate::Round,
      ) -> ::core::option::Option<Self> {
        self.0.checked_mul(factor, mode).map(Self)
      }

      /// Reads ASCII decimal text exactly. See the base type's
      /// `from_ascii`.
      pub const fn from_ascii(
        bytes: &[u8],
      ) -> ::core::result::Result<Self, $crate::ParseError> {
        match <$base>::from_ascii(bytes) {
          ::core::result::Result::Ok(v) => ::core::result::Result::Ok(Self(v)),
          ::core::result::Result::Err(e) => ::core::result::Result::Err(e),
        }
      }

      /// Writes the shortest exact text into `out`. See the base type's
      /// `write_ascii`.
      #[inline]
      pub fn write_ascii(
        self,
        out: &mut [u8],
      ) -> ::core::result::Result<usize, $crate::BufferTooSmall> {
        self.0.write_ascii(out)
      }

      /// Writes exactly `places` decimal places into `out`. See the base
      /// type's `write_ascii_dp`.
      #[inline]
      pub fn write_ascii_dp(
        self,
        out: &mut [u8],
        places: u32,
        mode: $crate::Round,
      ) -> ::core::result::Result<usize, $crate::BufferTooSmall> {
        self.0.write_ascii_dp(out, places, mode)
      }

      /// The shortest exact text, in a small stack buffer.
      #[must_use]
      #[inline]
      pub fn to_ascii(self) -> $crate::AsciiBuf {
        self.0.to_ascii()
      }
    }

    impl ::core::convert::From<$name> for $base {
      #[inline]
      fn from(v: $name) -> $base {
        v.0
      }
    }

    impl ::core::convert::From<$int> for $name {
      #[inline]
      fn from(n: $int) -> Self {
        Self(<$base>::from(n))
      }
    }

    impl ::core::ops::Add for $name {
      type Output = Self;

      #[inline]
      #[track_caller]
      fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
      }
    }

    impl ::core::ops::Sub for $name {
      type Output = Self;

      #[inline]
      #[track_caller]
      fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
      }
    }

    impl ::core::ops::AddAssign for $name {
      #[inline]
      #[track_caller]
      fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
      }
    }

    impl ::core::ops::SubAssign for $name {
      #[inline]
      #[track_caller]
      fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
      }
    }

    impl ::core::ops::Mul<$int> for $name {
      type Output = Self;

      #[inline]
      #[track_caller]
      fn mul(self, n: $int) -> Self {
        Self(self.0 * n)
      }
    }

    impl ::core::ops::Mul<$name> for $int {
      type Output = $name;

      #[inline]
      #[track_caller]
      fn mul(self, rhs: $name) -> $name {
        $name(rhs.0 * self)
      }
    }

    impl ::core::iter::Sum for $name {
      #[track_caller]
      fn sum<I: ::core::iter::Iterator<Item = Self>>(iter: I) -> Self {
        Self(iter.map(|v| v.0).sum())
      }
    }

    impl<'a> ::core::iter::Sum<&'a $name> for $name {
      #[track_caller]
      fn sum<I: ::core::iter::Iterator<Item = &'a Self>>(iter: I) -> Self {
        Self(iter.map(|v| v.0).sum())
      }
    }

    impl ::core::fmt::Display for $name {
      fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        ::core::fmt::Display::fmt(&self.0, f)
      }
    }

    impl ::core::fmt::Debug for $name {
      fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        ::core::write!(f, "{}({})", ::core::stringify!($name), self.0)
      }
    }

    impl ::core::str::FromStr for $name {
      type Err = $crate::ParseError;

      fn from_str(s: &str) -> ::core::result::Result<Self, Self::Err> {
        Self::from_ascii(s.as_bytes())
      }
    }

    $crate::__newtype_features! { $name, $base }
  };
}

/// Integration hook: extra impls for `newtype!` types, depending on
/// decimix's own Cargo features.
///
/// Because this macro is defined inside decimix, a `#[cfg(feature = ...)]` on
/// its definition is evaluated against decimix's features, not the calling
/// crate's. Future integrations (serde, Selecta, Cap'n Proto) add a
/// feature-gated definition here. Nothing is generated yet.
#[doc(hidden)]
#[macro_export]
macro_rules! __newtype_features {
  ($name:ident, $base:ty) => {};
}
