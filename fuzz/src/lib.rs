//! Shared helpers for the fuzz targets: an exact big-integer rounding oracle,
//! the same construction as the integration tests' (floor, then pick a
//! neighbour), independent of the crate's own rounding.

use decimix::Round;
use num_bigint::{BigInt, Sign};
use num_integer::Integer;

pub const MODES: [Round; 6] = [
  Round::HalfEven,
  Round::HalfAwayFromZero,
  Round::Floor,
  Round::Ceiling,
  Round::TowardZero,
  Round::AwayFromZero,
];

/// A rounding mode from one byte of fuzz input.
pub fn mode(b: u8) -> Round {
  MODES[b as usize % MODES.len()]
}

/// `n / d` rounded with `mode`, for any non-zero `d`.
pub fn round_div(n: &BigInt, d: &BigInt, mode: Round) -> BigInt {
  let (n, d) = if d.sign() == Sign::Minus {
    (-n, -d)
  } else {
    (n.clone(), d.clone())
  };
  let (down, rem) = n.div_mod_floor(&d);
  if rem == BigInt::ZERO {
    return down;
  }
  let up = &down + 1;
  let positive = n.sign() != Sign::Minus;
  let twice = &rem * 2;
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
    _ if twice < d => down,
    _ if twice > d => up,
    Round::HalfEven => {
      if down.is_even() {
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

pub fn pow10(k: u32) -> BigInt {
  BigInt::from(10).pow(k)
}
