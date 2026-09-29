//! Differential test of the scale-19 multiply kernels against a big-integer
//! oracle. Replaces the Python reference used during the design discussion.

use decimix::kernel::mul::{mul19, mul19_fast, mul19_floor};
use num_bigint::BigInt;
use num_integer::Integer;
use proptest::prelude::*;

const D: i128 = 10_000_000_000_000_000_000;

#[derive(Clone, Copy)]
enum Mode {
  HalfEven,
  Floor,
}

/// Exact `a * b / 10^19`, rounded, or `None` if it doesn't fit an i128.
fn oracle(a: i128, b: i128, mode: Mode) -> Option<i128> {
  let p = BigInt::from(a) * BigInt::from(b);
  let d = BigInt::from(D);
  let q = match mode {
    Mode::Floor => p.div_floor(&d),
    Mode::HalfEven => {
      let (mut q, r) = p.magnitude().div_rem(d.magnitude());
      let twice = &r * 2u32;
      if twice > *d.magnitude() || (twice == *d.magnitude() && q.is_odd()) {
        q += 1u32;
      }
      let q = BigInt::from(q);
      if p.sign() == num_bigint::Sign::Minus {
        -q
      } else {
        q
      }
    }
  };
  i128::try_from(q).ok()
}

/// Values biased towards the edges: ±1, powers of ten and their neighbours,
/// exact halves, whole numbers and the i128 limits.
fn value() -> impl Strategy<Value = i128> {
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

fn config() -> ProptestConfig {
  if cfg!(miri) {
    // Miri is ~1000x slower and has no filesystem access for persistence.
    ProptestConfig {
      cases: 32,
      failure_persistence: None,
      ..ProptestConfig::default()
    }
  } else {
    ProptestConfig::with_cases(20_000)
  }
}

proptest! {
  #![proptest_config(config())]

  #[test]
  fn mul19_half_even_matches_oracle(a in value(), b in value()) {
    prop_assert_eq!(mul19(a, b), oracle(a, b, Mode::HalfEven));
  }

  #[test]
  fn mul19_floor_matches_oracle(a in value(), b in value()) {
    prop_assert_eq!(mul19_floor(a, b), oracle(a, b, Mode::Floor));
  }

  #[test]
  fn mul19_fast_matches_oracle(a in value(), b in value()) {
    prop_assert_eq!(mul19_fast(a, b), oracle(a, b, Mode::HalfEven));
  }

  #[cfg(all(target_arch = "x86_64", not(miri)))]
  #[test]
  fn mul19_hw_matches_oracle(a in value(), b in value()) {
    prop_assert_eq!(
      decimix::kernel::mul::mul19_hw(a, b),
      oracle(a, b, Mode::HalfEven)
    );
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
}

#[test]
fn limits() {
  assert_eq!(mul19(i128::MAX, D), Some(i128::MAX));
  assert_eq!(mul19(i128::MIN, D), Some(i128::MIN));
  assert_eq!(mul19(i128::MIN, -D), None);
  assert_eq!(mul19_fast(i128::MIN, -D), None);
  assert_eq!(mul19(i128::MAX, 2 * D), None);
}
