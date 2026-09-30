//! Multiplication, division and sums of products through the public API,
//! checked against big-integer oracles in every rounding mode.

mod common;

use std::panic::catch_unwind;

use common::{D, MODES, config, round_div, uvalue, value};
use decimix::{Dec19, OutOfRange, ProductSum, Round, UDec19, dec, udec};
use num_bigint::BigInt;
use proptest::prelude::*;

fn big(v: i128) -> BigInt {
  BigInt::from(v)
}

fn dec_fits(v: &BigInt) -> Option<Dec19> {
  i128::try_from(v).ok().map(Dec19::from_raw)
}

fn udec_fits(v: &BigInt) -> Option<UDec19> {
  u128::try_from(v).ok().map(UDec19::from_raw)
}

/// `n / d` rounded, for any non-zero `d`: moves the sign onto the numerator
/// so the oracle's divisor is positive.
fn signed_round_div(n: BigInt, d: BigInt, mode: Round) -> BigInt {
  if d.sign() == num_bigint::Sign::Minus {
    round_div(&-n, &-d, mode)
  } else {
    round_div(&n, &d, mode)
  }
}

proptest! {
  #![proptest_config(config())]

  #[test]
  fn mul_matches_oracle(a in value(), b in value()) {
    let (x, y) = (Dec19::from_raw(a), Dec19::from_raw(b));
    for mode in MODES {
      let exact = round_div(&(big(a) * big(b)), &big(D), mode);
      let expected = dec_fits(&exact);
      prop_assert_eq!(x.checked_mul(y, mode), expected, "{:?}", mode);
      let saturated = expected.unwrap_or(
        if exact.sign() == num_bigint::Sign::Minus { Dec19::MIN } else { Dec19::MAX }
      );
      prop_assert_eq!(x.saturating_mul(y, mode), saturated, "{:?}", mode);
    }
  }

  #[test]
  fn mul_by_unsigned_matches_oracle(a in value(), b in uvalue()) {
    let (x, y) = (Dec19::from_raw(a), UDec19::from_raw(b));
    for mode in MODES {
      let exact = round_div(&(big(a) * BigInt::from(b)), &big(D), mode);
      prop_assert_eq!(x.checked_mul(y, mode), dec_fits(&exact), "{:?}", mode);
    }
  }

  #[test]
  fn udec19_mul_matches_oracle(a in uvalue(), b in uvalue()) {
    let (x, y) = (UDec19::from_raw(a), UDec19::from_raw(b));
    for mode in MODES {
      let exact =
        round_div(&(BigInt::from(a) * BigInt::from(b)), &big(D), mode);
      let expected = udec_fits(&exact);
      prop_assert_eq!(x.checked_mul(y, mode), expected, "{:?}", mode);
      prop_assert_eq!(
        x.saturating_mul(y, mode),
        expected.unwrap_or(UDec19::MAX),
        "{:?}",
        mode
      );
    }
  }

  #[test]
  fn div_matches_oracle(a in value(), b in value()) {
    let (x, y) = (Dec19::from_raw(a), Dec19::from_raw(b));
    for mode in MODES {
      let expected = (b != 0)
        .then(|| dec_fits(&signed_round_div(big(a) * big(D), big(b), mode)))
        .flatten();
      prop_assert_eq!(x.checked_div(y, mode), expected, "{:?}", mode);
    }
  }

  #[test]
  fn div_by_unsigned_matches_oracle(a in value(), b in uvalue()) {
    let (x, y) = (Dec19::from_raw(a), UDec19::from_raw(b));
    for mode in MODES {
      let expected = (b != 0)
        .then(|| dec_fits(&round_div(&(big(a) * big(D)), &BigInt::from(b), mode)))
        .flatten();
      prop_assert_eq!(x.checked_div(y, mode), expected, "{:?}", mode);
    }
  }

  #[test]
  fn udec19_div_matches_oracle(a in uvalue(), b in uvalue()) {
    let (x, y) = (UDec19::from_raw(a), UDec19::from_raw(b));
    for mode in MODES {
      let expected = (b != 0)
        .then(|| {
          udec_fits(&round_div(&(BigInt::from(a) * big(D)), &BigInt::from(b), mode))
        })
        .flatten();
      prop_assert_eq!(x.checked_div(y, mode), expected, "{:?}", mode);
    }
  }

  #[test]
  fn product_sum_matches_oracle(
    pairs in prop::collection::vec((value(), value()), 0..6),
    den in value(),
  ) {
    // The oracle total, and where (if anywhere) it leaves the 256-bit range.
    let (min, max) = (-(BigInt::from(1) << 255u32), (BigInt::from(1) << 255u32) - 1);
    let mut acc = ProductSum::new();
    let mut total = BigInt::ZERO;
    for &(a, b) in &pairs {
      let next = &total + big(a) * big(b);
      let result = acc.checked_add(Dec19::from_raw(a), Dec19::from_raw(b));
      if next < min || next > max {
        prop_assert_eq!(result, Err(OutOfRange));
        // The total is left unchanged; stop here.
        break;
      }
      prop_assert_eq!(result, Ok(()));
      total = next;
    }
    for mode in MODES {
      let finished = dec_fits(&round_div(&total, &big(D), mode)).ok_or(OutOfRange);
      prop_assert_eq!(acc.finish(mode), finished, "{:?}", mode);
      let divided = if den == 0 {
        Err(OutOfRange)
      } else {
        dec_fits(&signed_round_div(total.clone(), big(den), mode)).ok_or(OutOfRange)
      };
      prop_assert_eq!(acc.div(Dec19::from_raw(den), mode), divided, "{:?}", mode);
    }
  }
}

#[test]
fn examples() {
  let px = dec!(113.725);
  assert_eq!(
    px.mul(dec!(1234.5678901), Round::HalfEven),
    dec!(140401.2333016225)
  );
  assert_eq!(px.mul(udec!(250), Round::HalfEven), dec!(28431.25));
  assert_eq!(px.mul(dec!(-2), Round::HalfEven), dec!(-227.45));
  // 1/3, then rounding decides the last digit.
  assert_eq!(
    dec!(1).div(dec!(3), Round::HalfEven),
    dec!(0.3333333333333333333)
  );
  assert_eq!(
    dec!(2).div(dec!(3), Round::HalfEven),
    dec!(0.6666666666666666667)
  );
  assert_eq!(
    dec!(2).div(dec!(3), Round::TowardZero),
    dec!(0.6666666666666666666)
  );
  assert_eq!(
    dec!(-2).div(dec!(3), Round::Floor),
    dec!(-0.6666666666666666667)
  );
  assert_eq!(
    dec!(39805).div(udec!(350), Round::HalfEven),
    dec!(113.7285714285714285714)
  );
  assert_eq!(dec!(1).checked_div(Dec19::ZERO, Round::HalfEven), None);
  assert!(catch_unwind(|| dec!(1).div(Dec19::ZERO, Round::HalfEven)).is_err());
  assert!(catch_unwind(|| Dec19::MAX.mul(dec!(2), Round::HalfEven)).is_err());
  assert_eq!(
    Dec19::MAX.saturating_mul(dec!(-2), Round::HalfEven),
    Dec19::MIN
  );
  assert_eq!(udec!(1.5).mul(udec!(2), Round::HalfEven), udec!(3));
  assert_eq!(udec!(1).div(udec!(8), Round::HalfEven), udec!(0.125));
}

/// A UDec19 whole number of 2^64 or more is too big for the kernel's
/// whole-number shortcut, whose count is a u64. It used to be truncated
/// (2^64 became 0), which made this product 0.
#[test]
fn huge_unsigned_whole_numbers() {
  let two_64 = UDec19::from_raw((1u128 << 64) * D as u128);
  assert_eq!(udec!(1).mul(two_64, Round::HalfEven), two_64);
  assert_eq!(two_64.div(two_64, Round::HalfEven), udec!(1));
  assert_eq!(
    udec!(0.5).mul(two_64, Round::HalfEven).to_string(),
    "9223372036854775808"
  );
}

#[test]
fn product_sum_edges() {
  let mut acc = ProductSum::new();
  assert_eq!(acc.finish(Round::HalfEven), Ok(Dec19::ZERO));
  assert_eq!(acc.div(dec!(1), Round::HalfEven), Ok(Dec19::ZERO));
  assert_eq!(acc.div(Dec19::ZERO, Round::HalfEven), Err(OutOfRange));
  // Buys and sells net out exactly, with no rounding along the way.
  acc.add(dec!(0.1), dec!(0.1));
  acc.add(dec!(0.1), dec!(-0.1));
  assert_eq!(acc.finish(Round::HalfEven), Ok(Dec19::ZERO));
  // Each product here is below one 10^-19 step, and would round to zero
  // one at a time; the exact total doesn't.
  let tiny = Dec19::from_raw(1);
  let mut acc = ProductSum::new();
  for _ in 0..3 {
    acc.add(dec!(0.4), tiny);
  }
  assert_eq!(acc.finish(Round::HalfEven), Ok(Dec19::from_raw(1)));
  // Two products of the largest values overflow 256 bits.
  let mut acc = ProductSum::new();
  acc.add(UDec19::MAX, UDec19::from_raw(1 << 127));
  assert_eq!(acc.checked_add(UDec19::MAX, UDec19::MAX), Err(OutOfRange));
  assert_eq!(acc.finish(Round::HalfEven), Err(OutOfRange));
}
