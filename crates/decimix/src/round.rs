//! Rounding modes.

/// How to round a result that falls between two representable values.
///
/// Every operation that can round takes one of these as an argument: there is
/// no default and no hidden context. The table shows each mode rounding to a
/// whole number:
///
/// | Mode               |  2.4 |  2.5 |  3.5 | −2.5 | −2.4 |
/// |--------------------|-----:|-----:|-----:|-----:|-----:|
/// | `HalfEven`         |    2 |    2 |    4 |   −2 |   −2 |
/// | `HalfAwayFromZero` |    2 |    3 |    4 |   −3 |   −2 |
/// | `Floor`            |    2 |    2 |    3 |   −3 |   −3 |
/// | `Ceiling`          |    3 |    3 |    4 |   −2 |   −2 |
/// | `TowardZero`       |    2 |    2 |    3 |   −2 |   −2 |
/// | `AwayFromZero`     |    3 |    3 |    4 |   −3 |   −3 |
///
/// The first two only differ on exact halves. The last four ignore how close
/// the value is to either neighbour and always go one way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Round {
  /// To the nearest value; exact halves go to the even neighbour.
  ///
  /// Also called "banker's rounding". Halves go up as often as down, so a long
  /// run of rounded amounts doesn't drift. The usual choice for money.
  HalfEven,
  /// To the nearest value; exact halves go away from zero (2.5 → 3,
  /// −2.5 → −3).
  ///
  /// The "commercial" rounding taught in school and used by many fee
  /// schedules. Java and .NET call this `HALF_UP`/`AwayFromZero`; this name
  /// avoids the ambiguity of "up" for negative numbers.
  HalfAwayFromZero,
  /// Down, toward negative infinity (−2.5 → −3).
  Floor,
  /// Up, toward positive infinity (−2.5 → −2).
  Ceiling,
  /// Toward zero, i.e. drop the extra digits (−2.5 → −2).
  TowardZero,
  /// Away from zero (2.1 → 3, −2.1 → −3).
  AwayFromZero,
}

/// Decides whether a truncated result should be moved one step further from
/// zero.
///
/// The caller has divided a magnitude and got a quotient `q` (rounded toward
/// zero) and a remainder `r` out of divisor `d`, so the exact magnitude lies
/// `r/d` of the way from `q` to `q + 1`. `negative` is the sign of the final
/// result and `q_odd` whether `q` is odd. Returns true if the result should
/// be `q + 1` instead of `q`.
///
/// Requires `r < d`.
#[inline(always)]
pub(crate) const fn round_away(
  mode: Round,
  negative: bool,
  q_odd: bool,
  r: u128,
  d: u128,
) -> bool {
  // Compare r with the other part of the gap, d - r, instead of computing
  // 2r (which could overflow): r is more than half of d exactly when
  // r > d - r, and exactly half when they are equal.
  let other = d - r;
  decide(mode, negative, q_odd, r > other, r == other, r != 0)
}

/// The rounding decision itself, given how the remainder compares with half
/// the divisor. Callers that can compare more cheaply than
/// [`round_away`] (the multiply kernel, whose remainder is only 64 bits)
/// call this directly.
///
/// `above_half` and `at_half` say whether the remainder is more than, or
/// exactly, half the divisor; `inexact` whether it is non-zero at all.
///
/// Written without branches (`&`/`|` on bools, which always evaluate both
/// sides, instead of `&&`/`||`): it runs on every multiply, where a
/// mispredicted branch costs more than a few extra operations.
#[inline(always)]
pub(crate) const fn decide(
  mode: Round,
  negative: bool,
  q_odd: bool,
  above_half: bool,
  at_half: bool,
  inexact: bool,
) -> bool {
  match mode {
    // An exact value is never above or at half (0 is less than half of any
    // divisor), so the two "half" modes need no separate check.
    Round::HalfEven => above_half | (at_half & q_odd),
    Round::HalfAwayFromZero => above_half | at_half,
    // Toward -infinity: a negative result grows in magnitude, a positive one
    // is truncated.
    Round::Floor => negative & inexact,
    Round::Ceiling => !negative & inexact,
    Round::TowardZero => false,
    Round::AwayFromZero => inexact,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const MODES: [Round; 6] = [
    Round::HalfEven,
    Round::HalfAwayFromZero,
    Round::Floor,
    Round::Ceiling,
    Round::TowardZero,
    Round::AwayFromZero,
  ];

  /// The correctly rounded value of `n / d` (d > 0), worked out a different
  /// way from `round_away`: on the signed value, from its floor, with plain
  /// small-integer arithmetic.
  fn oracle(n: i64, d: i64, mode: Round) -> i64 {
    let down = n.div_euclid(d); // largest integer <= n/d
    let rem = n.rem_euclid(d); // n/d = down + rem/d, 0 <= rem < d
    if rem == 0 {
      return down;
    }
    let up = down + 1;
    let positive = n > 0;
    match mode {
      Round::Floor => down,
      Round::Ceiling => up,
      Round::TowardZero => {
        if positive {
          down
        } else {
          up
        }
      }
      Round::AwayFromZero => {
        if positive {
          up
        } else {
          down
        }
      }
      _ if 2 * rem < d => down,
      _ if 2 * rem > d => up,
      Round::HalfEven => {
        if down % 2 == 0 {
          down
        } else {
          up
        }
      }
      Round::HalfAwayFromZero => {
        if positive {
          up
        } else {
          down
        }
      }
    }
  }

  /// Every divisor d up to 255, every remainder r < d, quotients 0..4 (both
  /// parities), both signs and every mode: about 1.6 million cases, compared
  /// against exact arithmetic. The helper is the same code at every width,
  /// so trying every small case is strong evidence for all of them.
  #[test]
  fn round_away_exhaustive_8_bit() {
    // Miri is ~1000x slower; a smaller square still covers every branch.
    let max_d: u128 = if cfg!(miri) { 12 } else { 255 };
    for d in 1..=max_d {
      for r in 0..d {
        for q in 0..4u128 {
          for negative in [false, true] {
            for mode in MODES {
              let magnitude = (q * d + r) as i64;
              let n = if negative { -magnitude } else { magnitude };
              let expected = oracle(n, d as i64, mode);
              let away = round_away(mode, negative, q & 1 == 1, r, d);
              let rounded = (q + away as u128) as i64;
              let got = if negative { -rounded } else { rounded };
              assert_eq!(got, expected, "n={n} d={d} mode={mode:?}");
            }
          }
        }
      }
    }
  }
}
