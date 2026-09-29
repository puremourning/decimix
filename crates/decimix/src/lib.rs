//! # decimix
//!
//! Fixed-point decimal numbers for trading systems: fast, exact, and hard to
//! misuse.
//!
//! The main type is [`Dec19`]: a signed number with exactly **19 decimal
//! places**, stored as an `i128`. [`UDec19`] is the unsigned version, for
//! values that must never be negative.
//!
//! ## What "19 decimal places" means
//!
//! Every value is stored as a whole number of 10⁻¹⁹ steps
//! (0.0000000000000000001). 113.725 is stored as the integer
//! 1,137,250,000,000,000,000,000. So:
//!
//! - There are 19 digits **after the decimal point**, always. This is *not*
//!   SQL `DECIMAL(19)`, which means 19 digits in total.
//! - The range is about ±1.7 × 10¹⁹ whole units (0 to 3.4 × 10¹⁹ for
//!   [`UDec19`]), with 38 significant digits. That is more than any real
//!   price or tick needs, so nobody ever has to choose a scale or a
//!   multiplier.
//! - Every value has exactly one representation (`1.0` and `1.00` are the
//!   same thing), so `==`, `<` and hashing are plain integer operations.
//! - Adding, subtracting and comparing cost the same as for `i128`.
//!
//! ## A quick tour
//!
//! ```
//! use decimix::{Dec19, Round, dec};
//!
//! // Literals are checked at compile time and never pass through f64.
//! const TICK: Dec19 = dec!(0.005);
//! let bid = dec!(113.725);
//! let ask = dec!(113.74);
//!
//! // Exact operations are operators.
//! let spread = ask - bid;
//! assert_eq!(spread, dec!(0.015));
//! assert_eq!(spread * 2, dec!(0.03));
//! assert!(bid < ask);
//!
//! // Anything that can round is a method, and you must say how to round.
//! let mid = (bid + ask).div_int(2, Round::HalfEven);
//! assert_eq!(mid.round_to(TICK, Round::Floor), dec!(113.73));
//!
//! // Text in and out is exact.
//! assert_eq!(mid.to_string(), "113.7325");
//! assert_eq!("113.7325".parse::<Dec19>(), Ok(mid));
//! ```
//!
//! ## Rounding
//!
//! Multiplying two decimals, and any division, can produce more than 19
//! decimal places, so these are methods that take a [`Round`]:
//! [`Dec19::mul`], [`Dec19::div`], [`Dec19::div_int`], [`Dec19::round_to`],
//! [`Dec19::round_dp`] and so on. There is no default rounding and no hidden
//! context. See [`Round`] for what each mode does, with examples.
//!
//! Exact divisions that don't round have their own methods:
//! [`Dec19::div_floor`] (how many whole lots fit) and [`Dec19::rem_euclid`]
//! (what's left over). Both are correct for negative values, unlike Rust's
//! `/` on integers, which rounds toward zero.
//!
//! To add up many products (notional, a VWAP numerator), use [`ProductSum`]:
//! it keeps every product exactly and rounds once at the end.
//!
//! ## Floating point: only at the edges
//!
//! There is no `From<f64>`, no `as_f64` and no `Into<f64>`. Doubles can't hold
//! most decimals exactly (0.1 has no exact binary form, just as 1/3 has no
//! exact decimal one), and every calculation on them can add error.
//!
//! Many systems and APIs do use doubles for prices, though, so two
//! deliberately loud methods cross the boundary:
//!
//! - [`Dec19::to_f64_lossy`]: the nearest double, for output.
//! - [`Dec19::from_f64_lossy`]: reads a double, rounding to a `step` you
//!   choose: the finest precision you actually trust in it.
//!
//! They are for talking to other systems, **not for arithmetic**. If you need
//! maths this crate doesn't do, see below. Core crates can forbid them
//! entirely with Clippy, in their `clippy.toml`:
//!
//! ```toml
//! disallowed-methods = [
//!   { path = "decimix::Dec19::to_f64_lossy", reason = "floats only at I/O edges; use to_big() for maths" },
//!   { path = "decimix::Dec19::from_f64_lossy", reason = "floats only at I/O edges" },
//!   { path = "decimix::UDec19::to_f64_lossy", reason = "floats only at I/O edges; use to_big() for maths" },
//!   { path = "decimix::UDec19::from_f64_lossy", reason = "floats only at I/O edges" },
//! ]
//! ```
//!
//! Code at the edges (a gateway, an analytics crate) then opts in with
//! `#[allow(clippy::disallowed_methods)]`, which is easy to find in review.
//!
//! ## Maths this crate doesn't do, and what to use instead
//!
//! Square roots, logarithms, powers, statistics: turn on the `bigdecimal`
//! feature and convert with `to_big()`, which is always exact. Convert back
//! with `from_big(x, Round)`, which names its rounding, or `try_from`, which
//! refuses to round. Analytics code that genuinely wants doubles (e.g. for
//! `exp`) can use [`Dec19::to_f64_lossy`] under an explicit `#[allow]`.
//!
//! ## Domain types
//!
//! [`newtype!`] defines types like `Price` or `Qty` that wrap a `Dec19` or
//! `UDec19` and can't be mixed by accident. The `decimix-finance` crate
//! provides a standard set.
//!
//! ## What doesn't compile, on purpose
//!
//! No conversion from or to `f64`:
//!
//! ```compile_fail
//! let x = decimix::Dec19::from(1.5_f64);
//! ```
//!
//! ```compile_fail
//! let f: f64 = decimix::dec!(1.5).into();
//! ```
//!
//! The stored integer is not the value, so there is no `Into<i128>` (use
//! `to_raw` or `to_int`):
//!
//! ```compile_fail
//! let n: i128 = decimix::dec!(5).into();
//! ```
//!
//! Multiplying two decimals can round, so there is no `*` between them, and
//! no `/` or `%` at all (use `mul`, `div`, `div_floor`, `rem_euclid`):
//!
//! ```compile_fail
//! let (a, b) = (decimix::dec!(1.5), decimix::dec!(2));
//! let x = a * b;
//! ```
//!
//! ```compile_fail
//! let (a, b) = (decimix::dec!(1.5), decimix::dec!(2));
//! let x = a / b;
//! ```
//!
//! ```compile_fail
//! let (a, b) = (decimix::dec!(1.5), decimix::dec!(2));
//! let x = a % b;
//! ```
//!
//! Unsigned values can't be negated:
//!
//! ```compile_fail
//! let x = -decimix::udec!(1.5);
//! ```
//!
//! (Multiplying by an integer is exact, so that *is* an operator:
//! `dec!(1.5) * 2`.)
#![no_std]
#![warn(missing_docs)]

// Unit tests use std (proptest, formatting); the library itself does not.
#[cfg(test)]
extern crate std;

mod acc;
mod ascii;
#[cfg(feature = "bigdecimal")]
mod big;
mod common;
mod consts;
mod dec19;
mod error;
mod macros;
mod round;
mod udec19;

pub use crate::acc::ProductSum;
pub use crate::ascii::AsciiBuf;
pub use crate::dec19::Dec19;
#[cfg(feature = "bigdecimal")]
pub use crate::error::TryFromBigError;
pub use crate::error::{BufferTooSmall, FromF64Error, OutOfRange, ParseError};
pub use crate::round::Round;
pub use crate::udec19::UDec19;

/// Any of this crate's 19-decimal-place types: [`Dec19`] or [`UDec19`].
///
/// Used where either can appear, e.g. `price.mul(qty, round)` with a
/// `UDec19` quantity. Sealed: it can't be implemented outside this crate.
pub trait Fixed19: Copy + sealed::Sealed {
  /// Sign and magnitude (in 10⁻¹⁹ steps). Internal.
  #[doc(hidden)]
  fn __to_parts(self) -> (bool, u128);
}

mod sealed {
  pub trait Sealed {}
}

/// Low-level arithmetic kernels on raw scale-19 values.
///
/// Not part of the public API: public only so that the integration tests and
/// benchmarks can reach it.
#[doc(hidden)]
pub mod kernel {
  pub mod mul;
}
