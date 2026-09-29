//! Dividing one 19-decimal-place number by another.
//!
//! Both inputs count 10⁻¹⁹ steps, so dividing the stored integers would
//! cancel the units and give a plain ratio. To get the answer back in 10⁻¹⁹
//! steps, multiply the dividend by 10¹⁹ first: `(a × 10¹⁹) / b`. That
//! product can need about 192 bits, so this uses 256-bit integers from the
//! `ethnum` crate rather than a hand-written wide division (the design brief's
//! choice: general division is rare, and a hand-written one is where subtle
//! bugs live). Rounding is decided from the remainder.

use ethnum::U256;

use super::mul::{D, whole};
use crate::round::{Round, round_away};

/// `a / b` for magnitudes (no sign) at 19 decimal places, rounded with
/// `mode`, or `None` if `b` is zero or the result doesn't fit in 128 bits.
/// `negative` is the sign the final result will have.
#[inline]
pub(crate) fn div_parts(
  a: u128,
  b: u128,
  negative: bool,
  mode: Round,
) -> Option<u128> {
  if b == 0 {
    return None;
  }

  // Shortcut: dividing by a whole number n (e.g. a total quantity of 350).
  // Then (a × 10¹⁹) / (n × 10¹⁹) is just a / n: the 10¹⁹s cancel, and a
  // 128-by-64-bit division is much cheaper than a 256-bit one.
  if let Some(n) = whole(b) {
    let n = n as u128;
    let q = a / n;
    let away = round_away(mode, negative, q & 1 == 1, a % n, n);
    // q + 1 can't overflow: if n is 1 the remainder is 0 and nothing is
    // added; otherwise q is at most half the u128 range.
    return Some(q + away as u128);
  }

  // General case: (a × 10¹⁹) / b in 256 bits. a is below 2¹²⁸ and 10¹⁹ is
  // below 2⁶⁴, so the product is below 2¹⁹² and can't overflow.
  let num = U256::from(a) * U256::from(D);
  let (q, r) = num.div_rem(U256::from(b));
  let (q_high, q) = q.into_words();
  if q_high != 0 {
    return None;
  }
  // The remainder is below b, so it fits in 128 bits.
  let (_, r) = r.into_words();
  q.checked_add(round_away(mode, negative, q & 1 == 1, r, b) as u128)
}
