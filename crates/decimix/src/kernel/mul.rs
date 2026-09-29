//! Multiplying two 19-decimal-place numbers.
//!
//! Both inputs are stored as integers counting 10⁻¹⁹ steps, so their plain
//! integer product counts 10⁻³⁸ steps: 38 decimal places. To get back to 19
//! places we divide that product by 10¹⁹ (10,000,000,000,000,000,000) and
//! round. Three problems:
//!
//! 1. The product of two 128-bit numbers needs up to 256 bits. We build it
//!    from four 64 × 64-bit multiplications (`mul_128x128`).
//! 2. Dividing by 10¹⁹ with the CPU's divide instruction is slow, and a
//!    256-bit dividend is too wide for it anyway. Instead we divide in two
//!    64-bit "digits", like long division by hand, and each step replaces
//!    the division by multiplications (`div2by1`).
//! 3. Rounding is decided from the remainder, by the crate's shared rounding
//!    decision (`round::decide`).
//!
//! There is also a shortcut: when one input is a whole number (e.g. a
//! quantity of 250), no division is needed at all (`whole`,
//! `mul19_fast`).
//!
//! Checked against a big-integer oracle in `tests/mul_oracle.rs`, in every
//! rounding mode. The algorithm was imported from the design discussion (see
//! `docs/design-brief.md`); since then only the rounding step has changed
//! (it now supports every mode), and the comments.

use crate::round::{Round, decide};

/// 10¹⁹ = 10,000,000,000,000,000,000: the divisor, and the stored value of 1.
///
/// It fits in 64 bits with its top bit set (it's above 2⁶³ ≈ 9.2 × 10¹⁸).
/// The fast division below relies on that.
pub const D: u64 = 10_000_000_000_000_000_000;

/// A precomputed stand-in for 1/10¹⁹, so that we can divide by multiplying.
///
/// Mathematically it is ⌊(2¹²⁸ − 1) / 10¹⁹⌋ − 2⁶⁴. The ⌊(2¹²⁸ − 1) / 10¹⁹⌋ part
/// is "2¹²⁸ divided by 10¹⁹": multiplying by it and keeping the top half is
/// like dividing by 10¹⁹. That number needs 65 bits, and its top bit is
/// always 1 (because 10¹⁹ has its top bit set). So we subtract 2⁶⁴ to drop
/// that bit, fitting it in 64 bits, and add the dropped part back in
/// `div2by1` with a cheap addition.
///
/// This is the "reciprocal" from Möller & Granlund, *Improved division by
/// invariant integers* (2011), for further reading.
const V: u64 = (u128::MAX / D as u128 - (1u128 << 64)) as u64;

/// Divides the two-word number `u1 × 2⁶⁴ + u0` by 10¹⁹, returning the
/// quotient and remainder. Requires `u1 < 10¹⁹`, which guarantees the
/// quotient fits in one 64-bit word.
///
/// No divide instruction: two multiplications and a couple of corrections.
#[inline(always)]
pub(crate) fn div2by1(u1: u64, u0: u64) -> (u64, u64) {
  // Estimate the quotient. Multiplying the top word by V is "top word divided
  // by 10¹⁹, times 2⁶⁴"; adding the whole number back supplies the 2⁶⁴ that
  // was taken out of V. The top 64 bits of this sum, plus one, estimate the
  // quotient; the paper proves the estimate is either right, one too big or
  // (rarely) one too small. The bottom 64 bits (q0) are kept to detect which.
  let q =
    (V as u128 * u1 as u128).wrapping_add(((u1 as u128) << 64) | u0 as u128);
  let (mut q1, q0) = (((q >> 64) as u64).wrapping_add(1), q as u64);

  // The remainder that goes with the estimate: dividend − estimate × 10¹⁹.
  // Only the low 64 bits are needed, because the true remainder is below
  // 10¹⁹ < 2⁶⁴; wrapping arithmetic gives exactly those bits.
  let mut r = u0.wrapping_sub(q1.wrapping_mul(D));

  // Estimate one too big: the remainder went "negative", which shows up as
  // a wrapped value larger than q0 (the paper's test). Take one 10¹⁹ back.
  if r > q0 {
    q1 = q1.wrapping_sub(1);
    r = r.wrapping_add(D);
  }
  // Estimate one too small (rare): the remainder is a whole 10¹⁹ or more.
  if r >= D {
    q1 += 1;
    r -= D;
  }
  (q1, r)
}

/// Multiplies two 128-bit numbers into a 256-bit result, returned as four
/// 64-bit words, least significant first.
///
/// Schoolbook multiplication with 64-bit "digits": split each input into a
/// high and a low half, multiply the four pairs (each 64 × 64-bit product
/// fits in 128 bits), and add them up in the right positions with carries.
#[inline(always)]
pub(crate) fn mul_128x128(x: u128, y: u128) -> [u64; 4] {
  let (x0, x1, y0, y1) = (
    x as u64 as u128,
    (x >> 64) as u64 as u128,
    y as u64 as u128,
    (y >> 64) as u64 as u128,
  );
  // low × low lands at word 0, the two cross products at word 1, high × high
  // at word 2.
  let (p00, p01, p10, p11) = (x0 * y0, x0 * y1, x1 * y0, x1 * y1);
  // Word 1 collects the top of low × low and the bottoms of both cross
  // products. Three numbers below 2⁶⁴ sum to less than 2¹²⁸: no overflow.
  let mid = (p00 >> 64) + (p01 as u64 as u128) + (p10 as u64 as u128);
  // Words 2 and 3: the carry out of word 1, the tops of the cross products,
  // and high × high. The whole product is below 2²⁵⁶, so what's left for the
  // top two words fits in 128 bits: no overflow.
  let hi = (mid >> 64) + (p01 >> 64) + (p10 >> 64) + p11;
  [p00 as u64, mid as u64, hi as u64, (hi >> 64) as u64]
}

/// `x × y / 10¹⁹` for magnitudes (no sign), rounded with `mode`, or `None`
/// if the result doesn't fit in 128 bits. `negative` is the sign the final
/// result will have, which the directed rounding modes need.
#[inline(always)]
pub(crate) fn mul_magnitude(
  x: u128,
  y: u128,
  negative: bool,
  mode: Round,
) -> Option<u128> {
  let [w0, w1, w2, w3] = mul_128x128(x, y);

  // The quotient fits in 128 bits exactly when the product is below
  // 10¹⁹ × 2¹²⁸, i.e. when its top two words, read as one number, are below
  // 10¹⁹. Since 10¹⁹ fits in one word, that means: word 3 is zero and word 2
  // is below 10¹⁹. This also meets div2by1's requirement for the first step.
  if w3 != 0 || w2 >= D {
    return None;
  }

  // Long division by 10¹⁹, one 64-bit word at a time, like dividing a
  // three-digit number by a one-digit number by hand: divide the top two
  // words, then bring the remainder down next to the last word and divide
  // again. Each remainder is below 10¹⁹, which keeps the next step valid.
  let (q1, r) = div2by1(w2, w1);
  let (q0, r) = div2by1(r, w0);
  let q = ((q1 as u128) << 64) | q0 as u128; // truncated toward zero

  // The remainder r out of 10¹⁹ says how far the exact answer lies between
  // q and q + 1. It is below 10¹⁹ < 2⁶⁴, so 2r fits easily in 128 bits and
  // "more than half" is simply 2r > 10¹⁹. (The shared `round_away` compares
  // r with d − r instead, which is safe for any size but measurably slower
  // here.)
  let twice = r as u128 * 2;
  let away = decide(
    mode,
    negative,
    q & 1 == 1,
    twice > D as u128,
    twice == D as u128,
    r != 0,
  );
  // Rounding up can carry past 128 bits, but only when q is all ones; see
  // the `rounding_up_past_128_bits_is_overflow` test.
  q.checked_add(away as u128)
}

/// Gives a sign to a magnitude as an `i128`, or `None` if it doesn't fit.
#[inline(always)]
fn to_signed(negative: bool, m: u128) -> Option<i128> {
  if negative {
    // The most negative i128, −2¹²⁷, has no positive counterpart, so a
    // magnitude of exactly 2¹²⁷ is allowed here but not below.
    if m > 1u128 << 127 {
      None
    } else {
      Some((m as i128).wrapping_neg())
    }
  } else {
    i128::try_from(m).ok()
  }
}

/// `a × b` for two signed stored values, rounded with `mode`. `None` on
/// overflow.
#[inline(always)]
pub fn mul19_round(a: i128, b: i128, mode: Round) -> Option<i128> {
  // Work on sizes and put the sign back at the end: negative exactly when
  // one input is negative.
  let negative = (a < 0) ^ (b < 0);
  let m = mul_magnitude(a.unsigned_abs(), b.unsigned_abs(), negative, mode)?;
  to_signed(negative, m)
}

/// `a × b`, rounded half-even. `None` on overflow.
#[inline]
pub fn mul19(a: i128, b: i128) -> Option<i128> {
  mul19_round(a, b, Round::HalfEven)
}

/// `a × b`, rounded toward negative infinity. `None` on overflow.
#[inline(never)]
pub fn mul19_floor(a: i128, b: i128) -> Option<i128> {
  mul19_round(a, b, Round::Floor)
}

// ---- Shortcut: one input is a whole number ---------------------------------

/// 5¹⁹ = 19,073,486,328,125. With 2¹⁹ it makes up 10¹⁹ = 2¹⁹ × 5¹⁹.
const P5: u128 = 19_073_486_328_125;

/// The number that "undoes" multiplying by `d`, when you only keep the low
/// 128 bits: `d × inverse` leaves 1 in the low 128 bits. Exists for any odd
/// `d`.
///
/// Found by Newton's method: each round of `x = x × (2 − d × x)` doubles the
/// number of correct low bits. Starting from `x = d` gives 3 correct bits (any
/// odd number times itself leaves 1 in the low 3 bits), so 7 rounds give
/// 3 × 2⁷ = 384 ≥ 128 correct bits.
const fn inv_mod_2_128(d: u128) -> u128 {
  let mut x = d;
  let mut i = 0;
  while i < 7 {
    x = x.wrapping_mul(2u128.wrapping_sub(d.wrapping_mul(x)));
    i += 1;
  }
  x
}

/// Multiplying by this undoes a multiplication by 5¹⁹ (in the low 128 bits).
const INV5: u128 = inv_mod_2_128(P5);

/// The largest number that can be multiplied by 5¹⁹ without overflowing 128
/// bits.
const LIM5: u128 = u128::MAX / P5;

/// If the stored value `b` is a whole number (e.g. 250.0), returns that
/// number (250), without dividing. Otherwise `None`.
///
/// A stored value is a whole number exactly when it's divisible by 10¹⁹ =
/// 2¹⁹ × 5¹⁹, i.e. by both 2¹⁹ and 5¹⁹:
/// - divisible by 2¹⁹ means its low 19 bits are zero: one AND;
/// - for 5¹⁹, multiply by `INV5`, which undoes a multiplication by 5¹⁹. If
///   the number really is 5¹⁹ × k, the result is exactly k, which is at most
///   `LIM5`. If it isn't a multiple, the result is a number above
///   `LIM5`: multiplying by INV5 just reshuffles all 128-bit values
///   one-to-one, and the small results are all taken by the multiples.
///
/// The number returned always fits in 64 bits: a stored value is below
/// 2¹²⁸, so its whole part is below 2¹²⁸ / 10¹⁹ < 2⁶⁴.
#[inline(always)]
pub(crate) fn whole(b: u128) -> Option<u64> {
  if b & ((1 << 19) - 1) != 0 {
    return None;
  }
  let k = (b >> 19).wrapping_mul(INV5);
  if k > LIM5 {
    return None;
  }
  Some(k as u64)
}

/// `x × n` for a magnitude and a whole number, or `None` if it doesn't fit
/// in 128 bits. Exact: no rounding needed.
#[inline(always)]
pub(crate) fn mul_whole(x: u128, n: u64) -> Option<u128> {
  // 128 × 64-bit schoolbook multiplication: low half × n, then high half × n
  // plus the carry. If the high part needs more than 64 bits, the product
  // needs more than 128.
  let p0 = (x as u64 as u128) * n as u128;
  let mid = ((x >> 64) as u64 as u128) * n as u128 + (p0 >> 64);
  if mid >> 64 != 0 {
    return None;
  }
  Some((mid << 64) | (p0 as u64 as u128))
}

/// `a × b`, rounded with `mode`, taking the shortcut when `b` is a whole
/// number. `None` on overflow.
#[inline(always)]
pub fn mul19_fast_round(a: i128, b: i128, mode: Round) -> Option<i128> {
  let negative = (a < 0) ^ (b < 0);
  if let Some(n) = whole(b.unsigned_abs()) {
    // Price × whole quantity: exact, so the rounding mode doesn't matter.
    return to_signed(negative, mul_whole(a.unsigned_abs(), n)?);
  }
  let m = mul_magnitude(a.unsigned_abs(), b.unsigned_abs(), negative, mode)?;
  to_signed(negative, m)
}

/// `a × b`, rounded half-even, taking the shortcut when `b` is a whole
/// number. `None` on overflow.
#[inline]
pub fn mul19_fast(a: i128, b: i128) -> Option<i128> {
  mul19_fast_round(a, b, Round::HalfEven)
}
