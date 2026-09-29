//! Reading and writing decimal text.
//!
//! Everything works on sign-and-magnitude pairs: `(negative, magnitude)`,
//! where the magnitude counts 10⁻¹⁹ steps. The typed wrappers in `common.rs`
//! convert to and from `Dec19`/`UDec19` and check the range.
//!
//! This is the hot path for FIX and JSON, so both directions avoid slow
//! operations:
//! - Writing splits the value into its whole part and its 19-digit fraction
//!   with the multiply kernel's divide-by-10¹⁹ (no division instruction), then
//!   writes digits two at a time from a table.
//! - Reading checks and converts digits eight at a time, using ordinary
//!   64-bit arithmetic on eight bytes at once ("SWAR": SIMD within a
//!   register). Short numbers, the common case, take a plain loop.
//!
//! A simple one-digit-at-a-time parser (`parse_scalar`) is kept for `dec!`
//! literals, which may contain `_`, and as a reference in the tests.

use core::fmt;
use core::ops::Deref;

use crate::consts::{ONE_RAW, POW10};
use crate::error::{BufferTooSmall, ParseError};
use crate::kernel::mul::{D, div2by1};
use crate::round::{Round, decide, round_away};

// ---- Reading ---------------------------------------------------------------

/// What the parsers return: a sign and a magnitude in 10⁻¹⁹ steps, and what
/// was dropped beyond the 19th decimal place (only possible when no rounding
/// mode was given).
///
/// The caller checks its own type's range first, then whether anything was
/// dropped, so that an out-of-range number is reported as such even if it is
/// also too precise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Parsed {
  pub(crate) negative:  bool,
  pub(crate) magnitude: u128,
  pub(crate) dropped:   Dropped,
}

/// Digits dropped beyond the 19th decimal place, as a fraction of one 10⁻¹⁹
/// step, compared with one half. This is exactly what a later rounding step
/// needs to round the *original* number, not an already-rounded one (see
/// `ieee::round_to_step`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Dropped {
  /// Nothing, or only zeros.
  Nothing,
  /// Something, but less than half a step ("4999…").
  BelowHalf,
  /// Exactly half a step ("5", "50000").
  Half,
  /// More than half a step ("5001", "6").
  AboveHalf,
}

impl Dropped {
  /// From the first dropped digit, and whether any digit after it is
  /// non-zero.
  const fn new(first: u8, sticky: bool) -> Self {
    match (first, sticky) {
      (0, false) => Dropped::Nothing,
      (0..=4, _) => Dropped::BelowHalf,
      (5, false) => Dropped::Half,
      _ => Dropped::AboveHalf,
    }
  }
}

/// Reads `bytes[start..end]` as `[+-]digits[.digits]`.
///
/// With `round` set to `None`, digits past the 19th decimal place are dropped
/// and described by [`Parsed::dropped`]; otherwise they are rounded with that
/// mode.
///
/// Fails with [`ParseError::Invalid`] if the text is not a number at all,
/// and with [`ParseError::OutOfRange`] if the magnitude doesn't even fit in a
/// u128. The caller then checks its own range, and only then whether
/// anything was dropped, so the order is always Invalid, OutOfRange,
/// TooPrecise.
pub(crate) const fn parse(
  b: &[u8],
  start: usize,
  end: usize,
  round: Option<Round>,
) -> Result<Parsed, ParseError> {
  let mut i = start;
  let mut negative = false;
  if i < end && (b[i] == b'-' || b[i] == b'+') {
    negative = b[i] == b'-';
    i += 1;
  }

  // Find the whole part, the point and the fraction, converting digits as
  // they are scanned. The values are exact whenever the part has at most 19
  // significant digits, which covers everything except a few very large
  // numbers and more than 19 decimal places; those are redone below.
  let int_start = i;
  let (int_end, int_value) = scan_digits(b, i, end);
  i = int_end;
  let mut frac_start = i;
  let mut frac_end = i;
  let mut frac_value = 0;
  if i < end && b[i] == b'.' {
    frac_start = i + 1;
    (frac_end, frac_value) = scan_digits(b, frac_start, end);
    i = frac_end;
  }
  // Anything left over (a second point, a letter, a space) or no digits at
  // all (empty, "-", ".") is not a number.
  if i != end || (int_end == int_start && frac_end == frac_start) {
    return Err(ParseError::Invalid);
  }

  // Whole part.
  let mut magnitude = if int_end - int_start <= 19 {
    // Below 10¹⁹, so times 10¹⁹ it is below 10³⁸: fits in a u128.
    int_value as u128 * ONE_RAW
  } else {
    // Long: skip leading zeros and look again.
    let mut s = int_start;
    while s < int_end && b[s] == b'0' {
      s += 1;
    }
    let int_len = int_end - s;
    if int_len <= 19 {
      digits_to_u64(b, s, int_len) as u128 * ONE_RAW
    } else if int_len == 20 {
      // A 20-digit whole part fits in a u128 as a number, but may not once
      // multiplied by 10¹⁹. Only UDec19 can hold one, up to about 3.4 × 10¹⁹.
      let whole =
        (b[s] - b'0') as u128 * ONE_RAW + digits_to_u64(b, s + 1, 19) as u128;
      match whole.checked_mul(ONE_RAW) {
        Some(v) => v,
        None => return Err(ParseError::OutOfRange),
      }
    } else {
      return Err(ParseError::OutOfRange);
    }
  };

  // Fraction: the first 19 digits are exact. "113.725" has fraction digits
  // 725, which is 725 × 10¹⁶ steps of 10⁻¹⁹.
  let frac_len = frac_end - frac_start;
  if frac_len > 19 {
    frac_value = digits_to_u64(b, frac_start, 19);
  }
  let kept = if frac_len < 19 { frac_len } else { 19 };
  // Both factors fit in 64 bits, so this is a single 64 × 64-bit multiply.
  let frac = frac_value as u128 * POW10_U64[19 - kept] as u128;
  magnitude = match magnitude.checked_add(frac) {
    Some(v) => v,
    None => return Err(ParseError::OutOfRange),
  };

  // Digits beyond the 19th place: only matter if some are non-zero.
  if frac_len <= 19 {
    return Ok(Parsed {
      negative,
      magnitude,
      dropped: Dropped::Nothing,
    });
  }
  let first_extra = b[frac_start + 19] - b'0';
  let mut sticky = false;
  let mut j = frac_start + 20;
  while j < frac_end {
    sticky |= b[j] != b'0';
    j += 1;
  }
  round_extra(negative, magnitude, first_extra, sticky, round)
}

/// Applies digits beyond the 19th decimal place to a magnitude: describes
/// them without a rounding mode, otherwise rounds.
///
/// The dropped digits are a fraction of one 10⁻¹⁹ step. To round we only
/// need to compare that fraction with one half, which the first dropped digit
/// and a "sticky" flag (is anything after it non-zero?) tell us exactly: "5"
/// is exactly half (a tie), "50001" is above half and "4999" below.
const fn round_extra(
  negative: bool,
  magnitude: u128,
  first_extra: u8,
  sticky: bool,
  round: Option<Round>,
) -> Result<Parsed, ParseError> {
  let dropped = Dropped::new(first_extra, sticky);
  let Some(mode) = round else {
    return Ok(Parsed {
      negative,
      magnitude,
      dropped,
    });
  };
  let away = decide(
    mode,
    negative,
    magnitude & 1 == 1,
    matches!(dropped, Dropped::AboveHalf),
    matches!(dropped, Dropped::Half),
    !matches!(dropped, Dropped::Nothing),
  );
  let magnitude = if away {
    match magnitude.checked_add(1) {
      Some(v) => v,
      None => return Err(ParseError::OutOfRange),
    }
  } else {
    magnitude
  };
  Ok(Parsed {
    negative,
    magnitude,
    dropped: Dropped::Nothing,
  })
}

/// Eight bytes starting at `b[i]` as one 64-bit number. Little-endian, so
/// the first character is the lowest byte.
#[inline(always)]
const fn load8(b: &[u8], i: usize) -> u64 {
  u64::from_le_bytes([
    b[i],
    b[i + 1],
    b[i + 2],
    b[i + 3],
    b[i + 4],
    b[i + 5],
    b[i + 6],
    b[i + 7],
  ])
}

/// For eight characters packed into a u64, sets the top bit of every byte
/// that is *not* an ASCII digit, and clears everything else.
///
/// The digits '0'..'9' are the bytes 0x30..0x39. XOR with 0x30 turns them
/// into 0..9 and every other byte into something else. Then, per byte:
/// - adding 0x76 (118) takes any value of 10 or more to 0x80 or more, so
///   the top bit flags bytes that were above '9' or below '0';
/// - OR-ing in the XOR result itself flags bytes that already had the top
///   bit set (non-ASCII).
///
/// The addition can carry from one byte into the next, but only out of a
/// byte that is already flagged, and only upwards, into later characters.
/// Callers only look at the *first* flagged byte, which is never disturbed.
#[inline(always)]
const fn non_digits(chunk: u64) -> u64 {
  let t = chunk ^ 0x3030_3030_3030_3030;
  (t.wrapping_add(0x7676_7676_7676_7676) | t) & 0x8080_8080_8080_8080
}

/// Converts eight ASCII digits packed into a u64 into their value, e.g. the
/// bytes of "12345678" into 12,345,678.
///
/// Three multiply-and-shift steps, each combining neighbouring groups (the
/// first character is the lowest byte, and is the most significant digit):
/// 1. each pair of digits becomes a two-digit number: 1,2 → 12; 3,4 → 34;
///    5,6 → 56; 7,8 → 78. For every 16-bit lane, "×10 plus the next byte"
///    is `v * 10 + (v >> 8)`, and the mask keeps the low byte of each lane.
/// 2. each pair of those becomes a four-digit number: 12,34 → 1234;
///    56,78 → 5678 (`× 100` plus the next lane).
/// 3. the two halves become one number: 1234 × 10000 + 5678 = 12345678.
///
/// Overflow out of a lane only affects bits the mask then discards, so the
/// wrapping arithmetic is exact where it matters.
#[inline(always)]
const fn eight_digits(chunk: u64) -> u64 {
  let v = chunk.wrapping_sub(0x3030_3030_3030_3030); // each byte now 0..9
  let v = (v.wrapping_mul(10).wrapping_add(v >> 8)) & 0x00FF_00FF_00FF_00FF;
  let v = (v.wrapping_mul(100).wrapping_add(v >> 16)) & 0x0000_FFFF_0000_FFFF;
  (v.wrapping_mul(10_000).wrapping_add(v >> 32)) & 0xFFFF_FFFF
}

/// Scans the digits starting at `b[i]`: returns the index of the first
/// non-digit (or `end`), and the value of the digits.
///
/// The value is exact if there are at most 19 significant digits (leading
/// zeros are free: they leave it at 0). With more, the arithmetic wraps and
/// the value is meaningless, and the caller must not use it.
#[inline(always)]
const fn scan_digits(b: &[u8], mut i: usize, end: usize) -> (usize, u64) {
  let mut value = 0u64;
  // Eight at a time while there's room. If a chunk has a non-digit, the
  // lowest flagged byte is the first one; each byte is 8 bits, so its index
  // within the chunk is trailing zeros / 8.
  while i + 8 <= end {
    let chunk = load8(b, i);
    let flags = non_digits(chunk);
    if flags != 0 {
      break;
    }
    value = value
      .wrapping_mul(100_000_000)
      .wrapping_add(eight_digits(chunk));
    i += 8;
  }
  // The rest one at a time. Subtracting '0' maps the digits to 0..9 and
  // every other byte to 10 or more (wrapping below '0').
  while i < end {
    let d = b[i].wrapping_sub(b'0');
    if d > 9 {
      break;
    }
    value = value.wrapping_mul(10).wrapping_add(d as u64);
    i += 1;
  }
  (i, value)
}

/// The value of `len` ASCII digits starting at `b[s]`. Requires `len <= 19`
/// (so the value is below 10¹⁹ and fits in a u64) and only digits.
#[inline(always)]
const fn digits_to_u64(b: &[u8], s: usize, len: usize) -> u64 {
  let end = s + len;
  let mut acc = 0u64;
  let mut i = s;
  // After at most 11 digits acc is below 10¹¹, so acc × 10⁸ + 8 more digits
  // stays below 10¹⁹: no overflow.
  while i + 8 <= end {
    acc = acc * 100_000_000 + eight_digits(load8(b, i));
    i += 8;
  }
  while i < end {
    acc = acc * 10 + (b[i] - b'0') as u64;
    i += 1;
  }
  acc
}

/// Reads `bytes[start..end]` one character at a time, as [`parse`] does,
/// optionally allowing `_` between digits as in Rust literals.
///
/// Used for `dec!`/`udec!` literals (compile time, so speed doesn't matter)
/// and as a reference implementation in tests. Must give exactly the same
/// results as [`parse`] when `underscores` is false.
pub(crate) const fn parse_scalar(
  b: &[u8],
  start: usize,
  end: usize,
  underscores: bool,
  round: Option<Round>,
) -> Result<Parsed, ParseError> {
  let mut i = start;
  let mut negative = false;
  if i < end && (b[i] == b'-' || b[i] == b'+') {
    negative = b[i] == b'-';
    i += 1;
  }

  // Whole part. Overflow is remembered rather than reported at once, so
  // that malformed text is reported as Invalid first, as in `parse`.
  let mut int: u128 = 0;
  let mut overflow = false;
  let mut digits = 0usize;
  while i < end {
    let c = b[i];
    if c.is_ascii_digit() {
      match int.checked_mul(10) {
        Some(v) => match v.checked_add((c - b'0') as u128) {
          Some(v) => int = v,
          None => overflow = true,
        },
        None => overflow = true,
      }
      digits += 1;
    } else if !(c == b'_' && underscores) {
      break;
    }
    i += 1;
  }

  // Fraction: keep the first 19 digits exactly; beyond that keep only what
  // rounding needs (see `round_extra`).
  let mut frac: u128 = 0;
  let mut frac_len = 0usize;
  let mut extra_digits = 0usize;
  let mut first_extra: u8 = 0;
  let mut sticky = false;
  if i < end && b[i] == b'.' {
    i += 1;
    while i < end {
      let c = b[i];
      if c.is_ascii_digit() {
        let d = c - b'0';
        if frac_len < 19 {
          frac = frac * 10 + d as u128;
          frac_len += 1;
        } else {
          if extra_digits == 0 {
            first_extra = d;
          } else if d != 0 {
            sticky = true;
          }
          extra_digits += 1;
        }
        digits += 1;
      } else if !(c == b'_' && underscores) {
        break;
      }
      i += 1;
    }
  }

  if i != end || digits == 0 {
    return Err(ParseError::Invalid);
  }
  if overflow {
    return Err(ParseError::OutOfRange);
  }

  // magnitude = whole × 10¹⁹ + fraction scaled up to 19 places.
  let mut magnitude = match int.checked_mul(ONE_RAW) {
    Some(v) => v,
    None => return Err(ParseError::OutOfRange),
  };
  magnitude = match magnitude.checked_add(frac * POW10[19 - frac_len]) {
    Some(v) => v,
    None => return Err(ParseError::OutOfRange),
  };
  round_extra(negative, magnitude, first_extra, sticky, round)
}

/// Reads the source text of a `dec!`/`udec!` argument, as produced by
/// `stringify!`: a number literal (`113.725`, `-0.25`, `1_000`) or a string
/// literal (`"113.725"`).
///
/// Errors are `&'static str` because they become compile errors.
pub(crate) const fn parse_literal(s: &str) -> Result<Parsed, &'static str> {
  let b = s.as_bytes();
  let end = b.len();
  let mut i = 0;
  let mut negative = false;
  // A negative number literal arrives as "-" then the digits, possibly with
  // a space in between.
  if i < end && b[i] == b'-' {
    negative = true;
    i += 1;
    while i < end && b[i] == b' ' {
      i += 1;
    }
  }

  if i < end && b[i] == b'"' {
    if negative {
      return Err("put the minus sign inside the string, e.g. dec!(\"-1.5\")");
    }
    if end - i < 2 || b[end - 1] != b'"' {
      return Err("invalid string literal");
    }
    return match parse_scalar(b, i + 1, end - 1, true, None) {
      Ok(v) => Ok(v),
      Err(e) => Err(e.as_str()),
    };
  }

  let mut j = i;
  while j < end {
    let c = b[j];
    if c == b'e' || c == b'E' {
      return Err("exponents are not supported: write the number out in full");
    }
    if c.is_ascii_alphabetic() {
      return Err(
        "only plain decimal literals are allowed (no suffix such as f64, no \
         hex, octal or binary)",
      );
    }
    j += 1;
  }
  match parse_scalar(b, i, end, true, None) {
    // A second sign ("--1") is not a Rust literal, but be safe.
    Ok(Parsed { negative: true, .. }) => Err(ParseError::Invalid.as_str()),
    Ok(p) => Ok(Parsed { negative, ..p }),
    Err(e) => Err(e.as_str()),
  }
}

// ---- Writing ---------------------------------------------------------------

/// "00", "01", …, "99": the two ASCII digits of every number below 100, so
/// digits can be written two at a time (the approach the `itoa` crate uses).
const DIGIT_PAIRS: [u8; 200] = {
  let mut t = [0u8; 200];
  let mut i = 0;
  while i < 100 {
    t[2 * i] = b'0' + (i / 10) as u8;
    t[2 * i + 1] = b'0' + (i % 10) as u8;
    i += 1;
  }
  t
};

/// 10ᵏ for k from 0 to 19, as u64 (10¹⁹ is the largest that fits).
const POW10_U64: [u64; 20] = {
  let mut t = [1u64; 20];
  let mut k = 1;
  while k < 20 {
    t[k] = t[k - 1] * 10;
    k += 1;
  }
  t
};

/// Splits a magnitude into its whole part and its fraction (in 10⁻¹⁹ steps,
/// so always below 10¹⁹), i.e. `m / 10¹⁹` and `m % 10¹⁹`, without a division
/// instruction.
///
/// Uses the multiply kernel's divide-by-10¹⁹ step, which divides a two-word
/// number whose top word is below 10¹⁹. A u128's top word can be up to
/// 2⁶⁴ − 1, which is less than 2 × 10¹⁹, so first take out at most one 10¹⁹
/// from it by hand; that contributes 2⁶⁴ to the whole part. Then the
/// kernel divides the rest.
#[inline(always)]
pub(crate) fn split(m: u128) -> (u128, u64) {
  let hi = (m >> 64) as u64;
  let lo = m as u64;
  let (top, hi) = if hi >= D { (1u128, hi - D) } else { (0, hi) };
  let (q, r) = div2by1(hi, lo);
  ((top << 64) | q as u128, r)
}

/// The eight decimal digits of `n` (below 10⁸, with leading zeros) as the
/// values 0..9 in the eight bytes of a u64, first digit in the lowest byte
/// (so that writing it out little-endian puts it first). Add 0x30 to each
/// byte to get ASCII.
///
/// Instead of eight dependent divisions by 10, this splits all the digits
/// apart in three steps, working on several at once in separate lanes of the
/// u64:
/// 1. `n` into two 4-digit halves, in the two 32-bit lanes:
///    12345678 → 1234 | 5678.
/// 2. each 4-digit lane into two 2-digit numbers in 16-bit lanes:
///    1234 → 12 | 34. Dividing a 4-digit number by 100 is multiplying by
///    5243 and shifting right by 19 (exact for anything below 43,699), and
///    the product is small enough to stay inside its lane.
/// 3. each 2-digit lane into two digits in bytes: 12 → 1 | 2, using
///    "multiply by 103, shift right by 10" for dividing by 10 (exact below
///    179).
///
/// Each step's remainder is `value − quotient × divisor`, also lane by lane.
#[inline(always)]
const fn digits8(n: u64) -> u64 {
  let x = (n / 10_000) | ((n % 10_000) << 32);
  let q = ((x * 5243) >> 19) & 0x0000_007F_0000_007F;
  let x = q | ((x - q * 100) << 16);
  let q = ((x * 103) >> 10) & 0x000F_000F_000F_000F;
  q | ((x - q * 10) << 8)
}

/// ASCII '0' in every byte.
const ZEROS: u64 = 0x3030_3030_3030_3030;

/// Writes the digits of `v` (no leading zeros; "0" for zero) at `buf[pos..]`,
/// returning the position after them. `buf` needs 8 bytes of slack after
/// the digits, since whole 8-byte groups are stored at once.
#[inline(always)]
fn write_u64(buf: &mut [u8], pos: usize, v: u64) -> usize {
  if v < 100_000_000 {
    // At most 8 digits: convert all 8 (with leading zeros), then shift the
    // leading zeros out of the low end so the first real digit is first.
    let n = v.checked_ilog10().map_or(1, |l| l as usize + 1);
    let x = (digits8(v) + ZEROS) >> (8 * (8 - n));
    buf[pos..pos + 8].copy_from_slice(&x.to_le_bytes());
    pos + n
  } else {
    // Everything but the last 8 digits first, then those 8 in full.
    let pos = write_u64(buf, pos, v / 100_000_000);
    let x = digits8(v % 100_000_000) + ZEROS;
    buf[pos..pos + 8].copy_from_slice(&x.to_le_bytes());
    pos + 8
  }
}

/// Writes a fraction (below 10¹⁹, in 10⁻¹⁹ steps) as exactly 19 digits at
/// `buf[pos..pos + 19]`. Returns how many of those digits are trailing
/// zeros.
#[inline(always)]
fn write_frac19(buf: &mut [u8], pos: usize, frac: u64) -> usize {
  // 19 digits = 3 + 8 + 8. The three pieces are independent, so the CPU can
  // work on them in parallel.
  let a = frac / 10_000_000_000_000_000;
  let rest = frac % 10_000_000_000_000_000;
  let (b, c) = (rest / 100_000_000, rest % 100_000_000);
  let (xb, xc) = (digits8(b), digits8(c));
  let pair = (a % 100) as usize * 2;
  buf[pos] = b'0' + (a / 100) as u8;
  buf[pos + 1] = DIGIT_PAIRS[pair];
  buf[pos + 2] = DIGIT_PAIRS[pair + 1];
  buf[pos + 3..pos + 11].copy_from_slice(&(xb + ZEROS).to_le_bytes());
  buf[pos + 11..pos + 19].copy_from_slice(&(xc + ZEROS).to_le_bytes());
  // Trailing zero digits are zero bytes at the high end of the digit values
  // (the last digit is the highest byte), so count leading zero bits.
  if xc != 0 {
    xc.leading_zeros() as usize / 8
  } else if xb != 0 {
    8 + xb.leading_zeros() as usize / 8
  } else {
    16 + a.is_multiple_of(10) as usize + a.is_multiple_of(100) as usize
  }
}

/// Writes exactly `width` (at most 18) digits of `v`, with leading zeros, at
/// `buf[pos..pos + width]`. Requires `v < 10^width`, and 8 bytes of slack
/// after.
#[inline(always)]
fn write_fixed(buf: &mut [u8], pos: usize, v: u64, width: usize) {
  if width <= 8 {
    // `digits8` gives 8 digits with leading zeros; since v has at most
    // `width` digits, drop the extra leading zeros from the low end.
    let x = (digits8(v) + ZEROS) >> (8 * (8 - width));
    buf[pos..pos + 8].copy_from_slice(&x.to_le_bytes());
  } else {
    // The last 8 digits in full, everything before them recursively.
    write_fixed(buf, pos, v / 100_000_000, width - 8);
    let x = digits8(v % 100_000_000) + ZEROS;
    buf[pos + width - 8..pos + width].copy_from_slice(&x.to_le_bytes());
  }
}

/// Writes a whole part (at most about 3.4 × 10¹⁹) at `buf[pos..]`, returning
/// the position after it. Needs 8 bytes of slack after it.
#[inline(always)]
fn write_whole(buf: &mut [u8], pos: usize, int: u128) -> usize {
  if let Ok(v) = u64::try_from(int) {
    write_u64(buf, pos, v)
  } else {
    // Above 2⁶⁴ (only UDec19 gets here): exactly 20 digits, a leading digit
    // (1, 2 or 3) and then 19 more.
    buf[pos] = b'0' + (int / ONE_RAW) as u8;
    write_frac19(buf, pos + 1, (int % ONE_RAW) as u64);
    pos + 20
  }
}

/// Scratch space for formatting: sign, 20 whole digits, point, 19 fraction
/// digits is 41 bytes; the writers store whole 8-byte groups, so they need 8
/// bytes of slack beyond the text.
pub(crate) type Scratch = [u8; 49];

/// Copies a formatted number from scratch space into `out`.
#[inline(always)]
fn copy_out(buf: &[u8], out: &mut [u8]) -> Result<usize, BufferTooSmall> {
  let dest = out.get_mut(..buf.len()).ok_or(BufferTooSmall)?;
  dest.copy_from_slice(buf);
  Ok(buf.len())
}

/// Writes the shortest exact form into `out`: no trailing zeros, no point
/// for whole numbers.
#[inline]
pub(crate) fn write_shortest(
  negative: bool,
  magnitude: u128,
  out: &mut [u8],
) -> Result<usize, BufferTooSmall> {
  let mut buf: Scratch = [0; 49];
  let len = shortest(negative, magnitude, &mut buf);
  copy_out(&buf[..len], out)
}

/// Writes the shortest exact form into scratch space, returning its length.
#[inline(always)]
pub(crate) fn shortest(
  negative: bool,
  magnitude: u128,
  buf: &mut Scratch,
) -> usize {
  let (int, frac) = split(magnitude);
  let mut len = 0;
  if negative && magnitude != 0 {
    buf[0] = b'-';
    len = 1;
  }
  len = write_whole(buf, len, int);
  if frac != 0 {
    buf[len] = b'.';
    let trailing = write_frac19(buf, len + 1, frac);
    len += 20 - trailing;
  }
  len
}

/// Writes exactly `places` decimal places, rounding with `mode` if the value
/// has more.
pub(crate) fn write_places(
  negative: bool,
  magnitude: u128,
  places: u32,
  mode: Round,
  out: &mut [u8],
) -> Result<usize, BufferTooSmall> {
  let (mut int, frac) = split(magnitude);
  let places = places as usize;

  if places >= 19 {
    // Nothing to round: all 19 places, then zeros.
    let mut buf: Scratch = [0; 49];
    let mut len = 0;
    if negative && magnitude != 0 {
      buf[0] = b'-';
      len = 1;
    }
    len = write_whole(&mut buf, len, int);
    buf[len] = b'.';
    write_frac19(&mut buf, len + 1, frac);
    len += 20;
    let total = len + (places - 19);
    let dest = out.get_mut(..total).ok_or(BufferTooSmall)?;
    dest[..len].copy_from_slice(&buf[..len]);
    dest[len..].fill(b'0');
    return Ok(total);
  }

  // Keep `places` fraction digits: e.g. for 2 places, divide the 19-digit
  // fraction by 10¹⁷, leaving hundredths, and round using the remainder.
  let step = POW10_U64[19 - places];
  let mut kept = frac / step;
  let dropped = frac % step;
  // Parity for half-even is that of the whole rounded number. With at least
  // one place kept, the last digit is in `kept`; with none, it's `int`'s.
  let odd = if places == 0 {
    int & 1 == 1
  } else {
    kept & 1 == 1
  };
  if round_away(mode, negative, odd, dropped as u128, step as u128) {
    kept += 1;
    // 0.999 at 2 places rounds up to 1.00: carry into the whole part.
    if kept == POW10_U64[places] {
      kept = 0;
      int += 1;
    }
  }

  let mut buf: Scratch = [0; 49];
  let mut len = 0;
  // The sign is decided after rounding, so -0.001 at 2 places is "0.00".
  if negative && (int != 0 || kept != 0) {
    buf[0] = b'-';
    len = 1;
  }
  len = write_whole(&mut buf, len, int);
  if places > 0 {
    buf[len] = b'.';
    write_fixed(&mut buf, len + 1, kept, places);
    len += 1 + places;
  }
  copy_out(&buf[..len], out)
}

/// A formatted number held on the stack, from `to_ascii`.
///
/// Dereferences to `str`; `as_bytes` gives the ASCII bytes for a wire buffer.
#[derive(Clone, Copy)]
pub struct AsciiBuf {
  buf: Scratch,
  len: u8,
}

impl AsciiBuf {
  /// Formats straight into the buffer: no copy.
  #[inline]
  pub(crate) fn new(negative: bool, magnitude: u128) -> Self {
    let mut buf: Scratch = [0; 49];
    let len = shortest(negative, magnitude, &mut buf) as u8;
    Self { buf, len }
  }

  /// The text as a string slice.
  #[must_use]
  pub fn as_str(&self) -> &str {
    // Only ASCII digits, '-' and '.' are ever written.
    core::str::from_utf8(self.as_bytes()).expect("ASCII")
  }

  /// The text as ASCII bytes.
  #[must_use]
  pub fn as_bytes(&self) -> &[u8] {
    &self.buf[..self.len as usize]
  }
}

impl Deref for AsciiBuf {
  type Target = str;

  fn deref(&self) -> &str {
    self.as_str()
  }
}

impl fmt::Display for AsciiBuf {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.pad(self.as_str())
  }
}

impl fmt::Debug for AsciiBuf {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Debug::fmt(self.as_str(), f)
  }
}

/// `Display` for both types: shortest form, or rounded half-even to the
/// requested precision (`{:.2}`), with width, fill and `+` handled by
/// `Formatter::pad_integral`.
pub(crate) fn display(
  negative: bool,
  magnitude: u128,
  f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
  // Sign + 39 whole digits + point + up to 100 places.
  let mut buf = [0u8; 141];
  let len = match f.precision() {
    None => write_shortest(negative, magnitude, &mut buf),
    Some(p) if p <= 100 => {
      write_places(negative, magnitude, p as u32, Round::HalfEven, &mut buf)
    }
    Some(p) => {
      // Absurd precision: write it directly, without width/fill support.
      let len =
        write_places(negative, magnitude, 19, Round::HalfEven, &mut buf)
          .expect("fits");
      f.write_str(core::str::from_utf8(&buf[..len]).expect("ASCII"))?;
      for _ in 19..p {
        f.write_str("0")?;
      }
      return Ok(());
    }
  }
  .expect("buffer fits");
  let text = core::str::from_utf8(&buf[..len]).expect("ASCII");
  // pad_integral wants the digits without the sign.
  let (non_negative, digits) = match text.strip_prefix('-') {
    Some(rest) => (false, rest),
    None => (true, text),
  };
  f.pad_integral(non_negative, "", digits)
}

#[cfg(test)]
mod tests {
  use std::vec::Vec;
  use std::{format, vec};

  use proptest::prelude::*;

  use super::*;

  #[test]
  fn digits8_and_eight_digits_agree_with_std() {
    // Every value below 10⁸ in release builds; a spread sample in debug
    // builds and under Miri, where the full range is too slow.
    let step = if cfg!(miri) {
      999_983
    } else if cfg!(debug_assertions) {
      997
    } else {
      1
    };
    let mut n = 0u64;
    while n < 100_000_000 {
      let ascii = (digits8(n) + ZEROS).to_le_bytes();
      assert_eq!(&ascii, format!("{n:08}").as_bytes(), "digits8({n})");
      assert_eq!(eight_digits(u64::from_le_bytes(ascii)), n);
      n += step;
    }
    let last = 99_999_999u64;
    assert_eq!(eight_digits(digits8(last) + ZEROS), last);
  }

  #[test]
  fn non_digits_flags_the_first_non_digit_at_every_position() {
    for pos in 0..8 {
      for byte in 0..=255u8 {
        let mut chunk = *b"55555555";
        chunk[pos] = byte;
        let flags = non_digits(u64::from_le_bytes(chunk));
        if byte.is_ascii_digit() {
          assert_eq!(flags, 0, "byte {byte:#x} at {pos}");
        } else {
          assert_eq!(
            flags.trailing_zeros() / 8,
            pos as u32,
            "byte {byte:#x} at {pos}"
          );
        }
      }
    }
  }

  #[test]
  fn scan_digits_stops_at_every_position() {
    // One bad byte at every position of runs up to 24 long, including the
    // neighbours of '0'..'9' and bytes that make the SWAR addition carry.
    for len in 1..=24 {
      for pos in 0..len {
        for bad in [b'/', b':', b' ', b'.', b'_', 0x00, 0x80, 0xFA, 0xFF] {
          let mut text = Vec::from(&b"123456789012345678901234"[..len]);
          text[pos] = bad;
          let (end, _) = scan_digits(&text, 0, len);
          assert_eq!(end, pos, "len {len}, {bad:#x} at {pos}");
        }
      }
    }
  }

  /// Bytes that exercise the grammar and the SWAR edge cases.
  fn text_bytes() -> impl Strategy<Value = Vec<u8>> {
    let byte = prop_oneof![
      8 => b'0'..=b'9',
      2 => Just(b'.'),
      1 => Just(b'-'),
      1 => Just(b'+'),
      1 => prop::sample::select(vec![b'/', b':', b' ', b'e', b'_', 0, 0x80, 0xFA, 0xFF]),
    ];
    prop::collection::vec(byte, 0..48)
  }

  proptest! {
    #![proptest_config(ProptestConfig {
      cases: if cfg!(miri) { 8 } else { 20_000 },
      failure_persistence: None,
      ..ProptestConfig::default()
    })]

    /// The fast parser and the one-character-at-a-time parser must agree on
    /// everything, including which error they report.
    #[test]
    fn fast_parse_matches_scalar(text in text_bytes()) {
      let n = text.len();
      for round in [None, Some(Round::HalfEven), Some(Round::Floor)] {
        prop_assert_eq!(
          parse(&text, 0, n, round),
          parse_scalar(&text, 0, n, false, round)
        );
      }
    }

    #[test]
    fn fast_parse_matches_scalar_on_long_digit_runs(
      int in "[0-9]{0,40}",
      frac in proptest::option::of("[0-9]{0,40}"),
      neg in any::<bool>(),
    ) {
      let text = format!(
        "{}{}{}",
        if neg { "-" } else { "" },
        int,
        frac.map(|f| format!(".{f}")).unwrap_or_default()
      );
      let b = text.as_bytes();
      for round in [None, Some(Round::HalfEven), Some(Round::Ceiling)] {
        prop_assert_eq!(
          parse(b, 0, b.len(), round),
          parse_scalar(b, 0, b.len(), false, round)
        );
      }
    }
  }
}
