//! `ProductSum`: an exact running total of products.

use ethnum::{I256, U256};

use crate::kernel::mul::{div_d_round, mul_128x128};
use crate::round::round_away;
use crate::{Dec19, Fixed19, OutOfRange, Round};

/// An exact running total of products, such as Σ price × quantity.
///
/// Each product of two 19-place numbers has up to 38 decimal places. Instead
/// of rounding every product back to 19 places (and adding up the rounding
/// errors), this keeps every product exactly, in a 256-bit total, and rounds
/// once at the end. It is also faster than rounding each product.
///
/// ```
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
///
/// The total can reach about ±5.8 × 10³⁸ (whole units) before it overflows:
/// two products of the very largest `Dec19` values, and far beyond any real
/// notional.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProductSum {
  /// The total in 10⁻³⁸ steps.
  total: I256,
}

impl ProductSum {
  /// An empty total (zero).
  #[must_use]
  pub const fn new() -> Self {
    Self { total: I256::ZERO }
  }

  /// Adds `a × b` exactly. Either factor may be a [`Dec19`] or a
  /// [`UDec19`](crate::UDec19).
  ///
  /// # Panics
  ///
  /// If the total overflows. See [`checked_add`](Self::checked_add).
  #[track_caller]
  pub fn add<A: Fixed19, B: Fixed19>(&mut self, a: A, b: B) {
    self
      .checked_add(a, b)
      .expect("ProductSum overflow: total out of 256-bit range");
  }

  /// Adds `a × b` exactly, or leaves the total unchanged and fails if it
  /// would overflow.
  pub fn checked_add<A: Fixed19, B: Fixed19>(
    &mut self,
    a: A,
    b: B,
  ) -> Result<(), OutOfRange> {
    let (neg_a, a) = a.__to_parts();
    let (neg_b, b) = b.__to_parts();
    // The exact product in 10⁻³⁸ steps: two stored integers multiplied, no
    // division at all.
    let [w0, w1, w2, w3] = mul_128x128(a, b);
    let product = U256::from_words(
      ((w3 as u128) << 64) | w2 as u128,
      ((w1 as u128) << 64) | w0 as u128,
    );
    let total = if neg_a != neg_b {
      self.total.checked_sub_unsigned(product)
    } else {
      self.total.checked_add_unsigned(product)
    };
    self.total = total.ok_or(OutOfRange)?;
    Ok(())
  }

  /// The total, rounded to 19 places with `mode`.
  ///
  /// Fails if the result is outside [`Dec19`]'s range.
  pub fn finish(&self, mode: Round) -> Result<Dec19, OutOfRange> {
    let negative = self.total.is_negative();
    let (hi, lo) = self.total.unsigned_abs().into_words();
    let words = [lo as u64, (lo >> 64) as u64, hi as u64, (hi >> 64) as u64];
    // From 38 places back to 19: divide by 10¹⁹, as a multiplication does.
    let magnitude = div_d_round(words, negative, mode).ok_or(OutOfRange)?;
    Dec19::from_parts(negative, magnitude).ok_or(OutOfRange)
  }

  /// The total divided by `denominator`, rounded to 19 places with `mode`:
  /// for a VWAP, Σ(price × quantity) ÷ Σ quantity with a single rounding.
  ///
  /// Fails if `denominator` is zero or the result is outside [`Dec19`]'s
  /// range.
  pub fn div<D: Fixed19>(
    &self,
    denominator: D,
    mode: Round,
  ) -> Result<Dec19, OutOfRange> {
    let (neg_d, d) = denominator.__to_parts();
    if d == 0 {
      return Err(OutOfRange);
    }
    let negative = self.total.is_negative() != neg_d;
    // The total counts 10⁻³⁸ steps and the denominator 10⁻¹⁹ steps, so
    // dividing the stored integers leaves exactly 10⁻¹⁹ steps: no scaling.
    let (q, r) = self.total.unsigned_abs().div_rem(U256::from(d));
    let (q_high, q) = q.into_words();
    if q_high != 0 {
      return Err(OutOfRange);
    }
    // The remainder is below the denominator, so it fits in 128 bits.
    let (_, r) = r.into_words();
    let away = round_away(mode, negative, q & 1 == 1, r, d);
    let magnitude = q.checked_add(away as u128).ok_or(OutOfRange)?;
    Dec19::from_parts(negative, magnitude).ok_or(OutOfRange)
  }
}
