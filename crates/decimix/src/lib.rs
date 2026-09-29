//! # decimix
//!
//! Fixed-point decimal arithmetic for trading systems.
//!
//! The main type (to come) is `Dec19`: an `i128` mantissa at a fixed scale of
//! **19 decimal places**, i.e. `value = mantissa × 10⁻¹⁹`. That is 19 digits
//! *after the decimal point*, not 19 digits in total (not SQL `DECIMAL(19)`):
//! the range is about ±1.7×10¹⁹ whole units with 38 significant digits.
//!
//! See `docs/design-brief.md` in the repository for the design decisions.
#![warn(missing_docs)]

/// Low-level arithmetic kernels on raw scale-19 mantissas.
///
/// Not part of the public API: public only so that the integration tests and
/// benchmarks can reach it.
#[doc(hidden)]
pub mod kernel {
  pub mod mul;
}
