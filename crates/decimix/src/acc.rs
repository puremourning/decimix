//! `ProductSum`: an exact running total of products.

use crate::{Dec19, Fixed19, OutOfRange, Round};

/// An exact running total of products, such as Σ price × quantity.
///
/// Each product of two 19-place numbers has up to 38 decimal places. Instead
/// of rounding every product back to 19 places (and adding up the rounding
/// errors), this keeps every product exactly, in a 256-bit total, and rounds
/// once at the end. It is also faster than rounding each product.
///
/// ```no_run
/// use decimix::{ProductSum, Round, dec};
///
/// let fills = [(dec!(113.725), dec!(100)), (dec!(113.73), dec!(250))];
/// let mut acc = ProductSum::new();
/// for (px, qty) in fills {
///   acc.add(px, qty);
/// }
/// assert_eq!(acc.finish(Round::HalfEven), Ok(dec!(39805.0)));
/// // VWAP: the same total divided by the total quantity, still one rounding.
/// let vwap = acc.div(dec!(350), Round::HalfEven);
/// assert_eq!(vwap, Ok(dec!(113.7285714285714285714)));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProductSum {
  /// The total in 10⁻³⁸ steps, as four 64-bit words, least significant first,
  /// in two's complement.
  words: [u64; 4],
}

impl ProductSum {
  /// An empty total (zero).
  #[must_use]
  pub const fn new() -> Self {
    Self { words: [0; 4] }
  }

  /// Adds `a × b` exactly. Either factor may be a [`Dec19`] or a
  /// [`UDec19`](crate::UDec19).
  ///
  /// # Panics
  ///
  /// If the total overflows 256 bits, which takes on the order of 10⁷⁷
  /// maximum-size products.
  #[track_caller]
  pub fn add<A: Fixed19, B: Fixed19>(&mut self, a: A, b: B) {
    let _ = (a, b);
    todo!("phase 4: ProductSum::add")
  }

  /// The total, rounded to 19 places with `mode`.
  pub fn finish(&self, mode: Round) -> Result<Dec19, OutOfRange> {
    let _ = mode;
    todo!("phase 4: ProductSum::finish")
  }

  /// The total divided by `denominator`, rounded to 19 places with `mode`:
  /// for a VWAP, Σ(price × quantity) ÷ Σ quantity with a single rounding.
  ///
  /// Fails if `denominator` is zero or the result is out of range.
  pub fn div<D: Fixed19>(
    &self,
    denominator: D,
    mode: Round,
  ) -> Result<Dec19, OutOfRange> {
    let _ = (denominator, mode);
    todo!("phase 4: ProductSum::div")
  }
}
