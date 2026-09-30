//! Differential test of the scale-19 multiply kernels against a big-integer
//! oracle. Replaces the Python reference used during the design discussion.

mod common;

use common::{D, MODES, config, round_div, value};
use decimix::Round;
use decimix::kernel::mul::{
  mul19,
  mul19_fast,
  mul19_fast_round,
  mul19_floor,
  mul19_round,
};
use num_bigint::BigInt;
use proptest::prelude::*;

/// Exact `a * b / 10^19`, rounded, or `None` if it doesn't fit an i128.
fn oracle(a: i128, b: i128, mode: Round) -> Option<i128> {
  let p = BigInt::from(a) * BigInt::from(b);
  i128::try_from(round_div(&p, &BigInt::from(D), mode)).ok()
}

proptest! {
  #![proptest_config(config())]

  #[test]
  fn mul19_round_matches_oracle(a in value(), b in value()) {
    for mode in MODES {
      prop_assert_eq!(mul19_round(a, b, mode), oracle(a, b, mode), "{:?}", mode);
    }
  }

  #[test]
  fn mul19_fast_round_matches_oracle(a in value(), b in value()) {
    for mode in MODES {
      prop_assert_eq!(
        mul19_fast_round(a, b, mode),
        oracle(a, b, mode),
        "{:?}",
        mode
      );
    }
  }

  #[test]
  fn wrappers_match_oracle(a in value(), b in value()) {
    prop_assert_eq!(mul19(a, b), oracle(a, b, Round::HalfEven));
    prop_assert_eq!(mul19_floor(a, b), oracle(a, b, Round::Floor));
    prop_assert_eq!(mul19_fast(a, b), oracle(a, b, Round::HalfEven));
  }
}

#[test]
fn half_even_ties() {
  // 0.5 units of 10^-19 round to even: 5e-19 * 0.1 = 0.5e-19 -> 0.
  assert_eq!(mul19(5, D / 10), Some(0));
  // 15e-19 * 0.1 = 1.5e-19 -> 2e-19.
  assert_eq!(mul19(15, D / 10), Some(2));
  assert_eq!(mul19(-15, D / 10), Some(-2));
  assert_eq!(mul19_floor(-5, D / 10), Some(-1));
  // Exact halves in every mode: +-2.5e-19 and +-3.5e-19.
  let expect = [
    (Round::HalfEven, [2, 4, -2, -4]),
    (Round::HalfAwayFromZero, [3, 4, -3, -4]),
    (Round::Floor, [2, 3, -3, -4]),
    (Round::Ceiling, [3, 4, -2, -3]),
    (Round::TowardZero, [2, 3, -2, -3]),
    (Round::AwayFromZero, [3, 4, -3, -4]),
  ];
  for (mode, want) in expect {
    let got = [25, 35, -25, -35].map(|m| mul19_round(m, D / 10, mode).unwrap());
    assert_eq!(got, want, "{mode:?}");
  }
}

#[test]
fn limits() {
  assert_eq!(mul19(i128::MAX, D), Some(i128::MAX));
  assert_eq!(mul19(i128::MIN, D), Some(i128::MIN));
  assert_eq!(mul19(i128::MIN, -D), None);
  assert_eq!(mul19_fast(i128::MIN, -D), None);
  assert_eq!(mul19(i128::MAX, 2 * D), None);
}

/// The exact quotient here is 2^128 - 1 and it rounds up, so adding the
/// rounding step overflows 128 bits. The kernel as imported from the design
/// discussion wrapped that to 0 in release builds and returned `Some(0)`.
/// Found by searching for this case directly: random inputs essentially
/// never land in it.
#[test]
fn rounding_up_past_128_bits_is_overflow() {
  let (a, b) = (
    21_189_744_572_521_508_864,
    160_588_234_443_472_575_470_994_451_365_708_965_247,
  );
  assert_eq!(oracle(a, b, Round::HalfEven), None);
  for mode in MODES {
    assert_eq!(mul19_round(a, b, mode), None, "{mode:?}");
    assert_eq!(mul19_fast_round(a, b, mode), None, "{mode:?}");
  }
}
