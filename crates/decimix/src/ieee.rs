//! The explicit f64 boundary: `to_f64_lossy` and `from_f64_lossy`.
//!
//! Both work on sign-and-magnitude pairs (magnitude in 10⁻¹⁹ steps); the
//! typed wrappers in `common.rs` check their own range.
//!
//! Neither direction does any float arithmetic that could add error of its
//! own: going out, the result is the double nearest the exact value; coming
//! in, the double is read as the decimal every language prints for it, and
//! rounded once, exactly, in decimal.

use core::fmt::{self, Write};

use ethnum::U256;

use crate::FromF64Error;
use crate::ascii::{self, Dropped, Scratch};
use crate::consts::ONE_RAW;
use crate::round::{Round, decide};

/// 10ᵏ as f64 for k from 0 to 22. Every one of these is exactly
/// representable as a double (10²² = 2²² × 5²², and 5²² < 2⁵³), which is
/// what makes the fast path below exact.
const POW10_F64: [f64; 23] = [
  1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13,
  1e14, 1e15, 1e16, 1e17, 1e18, 1e19, 1e20, 1e21, 1e22,
];

/// The double nearest to `±magnitude × 10⁻¹⁹` (ties to even), i.e. exactly
/// what parsing the number's decimal text as a double would give.
pub(crate) fn to_f64(negative: bool, magnitude: u128) -> f64 {
  let value = to_f64_magnitude(magnitude);
  if negative { -value } else { value }
}

fn to_f64_magnitude(magnitude: u128) -> f64 {
  let (int, frac) = ascii::split(magnitude);
  if frac == 0 {
    // A whole number: Rust's integer-to-float `as` rounds to the nearest
    // double, ties to even.
    return int as f64;
  }

  // Drop the fraction's trailing zeros: 113.725 is stored with fraction
  // 7250000000000000000 (19 digits), which is 725 with 3 digits.
  // Take them off 8, then 4, 2 and 1 at a time: a handful of divisions
  // instead of up to 18.
  let mut frac = frac;
  let mut digits = 19;
  for (unit, n) in [(100_000_000, 8), (10_000, 4), (100, 2), (10, 1)] {
    while frac.is_multiple_of(unit) {
      frac /= unit;
      digits -= n;
    }
  }

  // Fast path: the whole number as an integer over a power of ten, e.g.
  // 113.725 = 113725 / 10³. If that integer is at most 2⁵³, it is exactly
  // representable as a double, and so is 10³ (see POW10_F64). IEEE division
  // of two exact values gives the nearest double to the true quotient, which
  // is exactly the answer wanted. This covers nearly every real price.
  let whole = int.checked_mul(crate::consts::POW10[digits]);
  if let Some(n) = whole.and_then(|w| w.checked_add(frac as u128))
    && n <= 1 << 53
  {
    return n as f64 / POW10_F64[digits];
  }

  // Otherwise let the standard library do it: write the exact decimal and
  // parse it, which also gives the nearest double. No allocation.
  let mut buf: Scratch = [0; 49];
  let len = ascii::shortest(false, magnitude, &mut buf);
  let text = core::str::from_utf8(&buf[..len]).expect("ASCII");
  text.parse().expect("decimal text always parses as f64")
}

/// A fixed-size text buffer that `write!` can write into.
struct TextBuf {
  buf: [u8; 80],
  len: usize,
}

impl Write for TextBuf {
  fn write_str(&mut self, s: &str) -> fmt::Result {
    let end = self.len + s.len();
    self
      .buf
      .get_mut(self.len..end)
      .ok_or(fmt::Error)?
      .copy_from_slice(s.as_bytes());
    self.len = end;
    Ok(())
  }
}

/// Reads a double as the shortest decimal that converts back to it, then
/// rounds that decimal to a multiple of `step` (a magnitude in 10⁻¹⁹ steps,
/// not zero) with `mode`. Returns the sign and magnitude of the result.
///
/// Fails if the double is NaN or infinite, or if the result doesn't fit in
/// a u128; the caller checks its own type's range.
pub(crate) fn from_f64(
  x: f64,
  step: u128,
  mode: Round,
) -> Result<(bool, u128), FromF64Error> {
  if !x.is_finite() {
    return Err(FromF64Error::NotFinite);
  }
  // Both Dec19 and UDec19 top out below 3.5 × 10¹⁹ whole units, and a step
  // is itself in range, so rounding a number of 10²⁰ or more to a step
  // lands at least 10²⁰ − 3.5 × 10¹⁹ away from zero: out of range.
  let a = x.abs();
  if a >= 1e20 {
    return Err(FromF64Error::OutOfRange);
  }
  let negative = x < 0.0; // -0.0 is zero, not negative

  let (magnitude, dropped) = if a < 1e-20 {
    // Below a tenth of the smallest step: Rust would print hundreds of
    // zeros, and all we need to know is that it's more than nothing (unless
    // it is zero) and less than half a step.
    let dropped = if a == 0.0 {
      Dropped::Nothing
    } else {
      Dropped::BelowHalf
    };
    (0, dropped)
  } else {
    // Rust's `{}` for f64 prints the shortest decimal that converts back to
    // exactly this double, never in exponent form. For values in
    // [10⁻²⁰, 10²⁰) with at most 17 significant digits that is under 60
    // characters.
    let mut text = TextBuf {
      buf: [0; 80],
      len: 0,
    };
    write!(text, "{a}").expect("fits in 80 bytes");
    let text = &text.buf[..text.len];

    if a >= 1e19 {
      // Big: the number itself may be out of range and yet round into it,
      // e.g. 3.45 × 10¹⁹ rounded down to a step of 5.7 × 10¹⁸ is
      // 2.87 × 10¹⁹. Its stored value (× 10¹⁹) can be too big for a u128, so
      // round it in 256 bits instead. Every double this large prints as a
      // whole number (e.g. "17014118346046923000"; note that its exact
      // binary value, 17014118346046922752, is not what we want).
      let whole = text.iter().fold(0u128, |n, &c| n * 10 + (c - b'0') as u128);
      return round_whole_to_step(negative, whole, step, mode)
        .map(|m| (negative, m))
        .ok_or(FromF64Error::OutOfRange);
    }

    // Rust's f64 formatting is always plain decimal, and below 10¹⁹ the
    // stored value fits in a u128, so this can't fail.
    let parsed = ascii::parse(text, 0, text.len(), None)
      .expect("plain decimal below 1e19");
    (parsed.magnitude, parsed.dropped)
  };

  let magnitude = round_to_step(negative, magnitude, dropped, step, mode)
    .ok_or(FromF64Error::OutOfRange)?;
  Ok((negative, magnitude))
}

/// Rounds a number to a multiple of `step`, once.
///
/// The number is `magnitude` 10⁻¹⁹ steps plus a dropped fraction of one more
/// step, described by `dropped`. Rounding it in one go (rather than first to
/// 19 places, then to `step`) avoids double rounding: 4.6 × 10⁻¹⁹ rounded to
/// 19 places is 5 × 10⁻¹⁹, which would then look like an exact tie for a
/// step of 10⁻¹⁸, though the original was below half and rounds down.
///
/// Returns `None` if the result doesn't fit in a u128.
fn round_to_step(
  negative: bool,
  magnitude: u128,
  dropped: Dropped,
  step: u128,
  mode: Round,
) -> Option<u128> {
  // magnitude = q × step + r, so the exact number is q steps plus r + e
  // smallest steps, where e (the dropped part) is between 0 and 1.
  let q = magnitude / step;
  let r = magnitude % step;
  // Compare r + e with half a step, without computing 2r (which could
  // overflow): r ≥ step − r means 2r ≥ step.
  let other = step - r;
  let (above, at) = match dropped {
    // Nothing dropped: the usual comparison.
    Dropped::Nothing => (r > other, r == other),
    // Something dropped (0 < e < 1). If 2r ≥ step, then 2(r + e) > step.
    // If 2r + 1 < step, then 2(r + e) < 2r + 2 ≤ step. The only close case is
    // 2r + 1 = step exactly (an odd step), where e itself decides: above,
    // at or below half.
    _ if r >= other => (true, false),
    _ if other - r == 1 => (
      matches!(dropped, Dropped::AboveHalf),
      matches!(dropped, Dropped::Half),
    ),
    _ => (false, false),
  };
  let inexact = r != 0 || !matches!(dropped, Dropped::Nothing);
  let away = decide(mode, negative, q & 1 == 1, above, at, inexact);
  q.checked_add(away as u128)?.checked_mul(step)
}

/// Rounds a whole number of units (not steps of 10⁻¹⁹) to a multiple of
/// `step` (in 10⁻¹⁹ steps), using 256-bit arithmetic because the number's
/// stored value can exceed a u128. Returns the result as a magnitude in
/// 10⁻¹⁹ steps, or `None` if that doesn't fit in a u128.
fn round_whole_to_step(
  negative: bool,
  whole: u128,
  step: u128,
  mode: Round,
) -> Option<u128> {
  let stored = U256::from(whole) * U256::from(ONE_RAW);
  let (q, r) = stored.div_rem(U256::from(step));
  let (q_high, q) = q.into_words();
  if q_high != 0 {
    return None;
  }
  // The remainder is below the step, so it fits in 128 bits.
  let (_, r) = r.into_words();
  let other = step - r;
  let away = decide(mode, negative, q & 1 == 1, r > other, r == other, r != 0);
  q.checked_add(away as u128)?.checked_mul(step)
}
