//! Shared test helpers: value generators biased to the edges, and exact
//! big-integer oracles that are deliberately built differently from the
//! crate's own code.

#![allow(dead_code)]

use decimix::Round;
use num_bigint::BigInt;
use num_integer::Integer;
use proptest::prelude::*;

/// 10^19: the stored value of 1.
pub const D: i128 = 10_000_000_000_000_000_000;

pub const MODES: [Round; 6] = [
  Round::HalfEven,
  Round::HalfAwayFromZero,
  Round::Floor,
  Round::Ceiling,
  Round::TowardZero,
  Round::AwayFromZero,
];

pub fn pow10(k: u32) -> BigInt {
  BigInt::from(10).pow(k)
}

/// `n / d` rounded with `mode`, for `d > 0`.
///
/// Built differently from the crate on purpose: it floors the signed value,
/// then picks between that and the next value up, instead of working on
/// magnitudes.
pub fn round_div(n: &BigInt, d: &BigInt, mode: Round) -> BigInt {
  let (down, rem) = n.div_mod_floor(d); // n = down * d + rem, 0 <= rem < d
  if rem == BigInt::ZERO {
    return down;
  }
  let up = &down + 1;
  let positive = n.sign() != num_bigint::Sign::Minus;
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
    _ if twice < *d => down,
    _ if twice > *d => up,
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

/// The exact text of a stored value, worked out with big integers and
/// string padding, independently of the crate's formatter: shortest form,
/// no trailing zeros.
pub fn reference_text(raw: &BigInt) -> String {
  let (int, frac) = raw.magnitude().div_rem(pow10(19).magnitude());
  let sign = if raw.sign() == num_bigint::Sign::Minus {
    "-"
  } else {
    ""
  };
  let frac = format!("{frac:019}");
  let frac = frac.trim_end_matches('0');
  if frac.is_empty() {
    format!("{sign}{int}")
  } else {
    format!("{sign}{int}.{frac}")
  }
}

/// A signed count of `10^-places` units as text with exactly `places`
/// decimal places, e.g. (-11372, 2) -> "-113.72".
pub fn fixed_text(units: &BigInt, places: u32) -> String {
  let sign = if units.sign() == num_bigint::Sign::Minus {
    "-"
  } else {
    ""
  };
  let digits = units.magnitude().to_string();
  let places = places as usize;
  if places == 0 {
    return format!("{sign}{digits}");
  }
  let digits = format!("{digits:0>width$}", width = places + 1);
  let (int, frac) = digits.split_at(digits.len() - places);
  format!("{sign}{int}.{frac}")
}

/// Signed stored values biased towards the edges: ±1, powers of ten and
/// their neighbours, exact halves, whole numbers and the i128 limits.
pub fn value() -> impl Strategy<Value = i128> {
  let pow10 = (0u32..=38, -1i128..=1, any::<bool>()).prop_map(|(e, d, neg)| {
    let v = 10i128.pow(e) + d;
    if neg { -v } else { v }
  });
  let small_times_pow10 =
    (-100i128..=100, 0u32..=36).prop_map(|(m, e)| m * 10i128.pow(e));
  let half = any::<i64>().prop_map(|k| k as i128 * D + D / 2);
  let whole = any::<i64>().prop_map(|k| k as i128 * D);
  let limits = prop_oneof![
    Just(0i128),
    Just(1),
    Just(-1),
    Just(i128::MAX),
    Just(i128::MIN),
    Just(i128::MAX - 1),
    Just(i128::MIN + 1),
  ];
  let moderate = -(10i128.pow(29))..10i128.pow(29);
  prop_oneof![
    3 => any::<i128>(),
    3 => moderate,
    2 => pow10,
    2 => small_times_pow10,
    1 => half,
    2 => whole,
    1 => limits,
  ]
}

/// Unsigned stored values biased towards the edges, including the range
/// above i128::MAX that only UDec19 has.
pub fn uvalue() -> impl Strategy<Value = u128> {
  let pow10 = (0u32..=38, -1i128..=1)
    .prop_map(|(e, d)| (10u128.pow(e) as i128 + d).max(0) as u128);
  let whole = any::<u64>().prop_map(|k| k as u128 * D as u128);
  let limits = prop_oneof![
    Just(0u128),
    Just(1),
    Just(u128::MAX),
    Just(u128::MAX - 1),
    Just(i128::MAX as u128),
    Just(i128::MAX as u128 + 1),
    Just(u64::MAX as u128 * D as u128),
  ];
  prop_oneof![
    3 => any::<u128>(),
    2 => 0u128..10u128.pow(29),
    2 => pow10,
    2 => whole,
    1 => limits,
  ]
}

pub fn config() -> ProptestConfig {
  if cfg!(miri) {
    // Miri is ~1000x slower and has no filesystem access for persistence.
    // It is here to catch undefined behaviour, which a few cases through
    // each path do as well as many; the oracle checks run natively.
    ProptestConfig {
      cases: 8,
      failure_persistence: None,
      ..ProptestConfig::default()
    }
  } else {
    ProptestConfig::with_cases(20_000)
  }
}
