//! The exact core: constants, conversions, operators, and the rounding
//! methods that don't multiply, checked against big-integer oracles.

mod common;

use std::panic::catch_unwind;

use common::{D, MODES, config, pow10, round_div, uvalue, value};
use decimix::{Dec19, OutOfRange, Round, UDec19, dec, udec};
use num_bigint::BigInt;
use num_integer::Integer;
use proptest::prelude::*;

fn dec_fits(v: &BigInt) -> Option<Dec19> {
  i128::try_from(v).ok().map(Dec19::from_raw)
}

fn udec_fits(v: &BigInt) -> Option<UDec19> {
  u128::try_from(v).ok().map(UDec19::from_raw)
}

#[test]
fn constants() {
  assert_eq!(Dec19::ZERO.to_raw(), 0);
  assert_eq!(Dec19::ONE.to_raw(), D);
  assert_eq!(Dec19::SMALLEST_STEP.to_raw(), 1);
  assert_eq!(Dec19::MAX.to_raw(), i128::MAX);
  assert_eq!(Dec19::MIN.to_raw(), i128::MIN);
  assert_eq!(UDec19::MAX.to_raw(), u128::MAX);
  assert_eq!(UDec19::MIN, UDec19::ZERO);
  assert_eq!(Dec19::DECIMAL_PLACES, 19);
  assert_eq!(Dec19::MAX_ASCII_LEN, Dec19::MIN.to_string().len());
  assert_eq!(UDec19::MAX_ASCII_LEN, UDec19::MAX.to_string().len());
  assert_eq!(Dec19::default(), Dec19::ZERO);
  assert_eq!(dec!(1.0), dec!(1.00)); // one representation per value
}

#[test]
fn integer_conversions() {
  assert_eq!(Dec19::from(250_i64), dec!(250));
  assert_eq!(Dec19::from(i64::MIN).to_raw(), i64::MIN as i128 * D);
  assert_eq!(Dec19::from(u32::MAX).to_raw(), u32::MAX as i128 * D);
  // The largest whole number a Dec19 holds is 17,014,118,346,046,923,173.
  assert!(Dec19::try_from(17_014_118_346_046_923_173_u64).is_ok());
  assert_eq!(
    Dec19::try_from(17_014_118_346_046_923_174_u64),
    Err(OutOfRange)
  );
  assert_eq!(Dec19::try_from(u64::MAX), Err(OutOfRange));
  assert_eq!(Dec19::try_from(-5_i128), Ok(dec!(-5)));
  assert_eq!(Dec19::try_from(u128::MAX), Err(OutOfRange));

  assert_eq!(
    UDec19::from(u64::MAX).to_raw(),
    u64::MAX as u128 * D as u128
  );
  assert_eq!(UDec19::try_from(-1_i64), Err(OutOfRange));
  assert_eq!(UDec19::try_from(7_i32), Ok(udec!(7)));
  assert!(UDec19::try_from(34_028_236_692_093_846_346_u128).is_ok());
  assert_eq!(
    UDec19::try_from(34_028_236_692_093_846_347_u128),
    Err(OutOfRange)
  );

  assert_eq!(UDec19::try_from(dec!(1.5)), Ok(udec!(1.5)));
  assert_eq!(UDec19::try_from(dec!(-0.1)), Err(OutOfRange));
  assert_eq!(Dec19::try_from(udec!(1.5)), Ok(dec!(1.5)));
  assert_eq!(Dec19::try_from(UDec19::MAX), Err(OutOfRange));
}

#[test]
fn raw_and_bytes() {
  let px = dec!(113.725);
  assert_eq!(px.to_raw(), 1_137_250_000_000_000_000_000);
  assert_eq!(Dec19::from_le_bytes(px.to_le_bytes()), px);
  assert_eq!(Dec19::from_be_bytes(px.to_be_bytes()), px);
  assert_eq!(px.to_le_bytes(), px.to_raw().to_le_bytes());
  let q = udec!(250);
  assert_eq!(UDec19::from_le_bytes(q.to_le_bytes()), q);
}

/// The key encoding is stored by other systems (sorted storage), so pin
/// its exact bytes as well as its ordering.
#[test]
fn key_bytes_are_fixed() {
  let mut minus_one = [0xFF; 16];
  minus_one[0] = 0x7F;
  let mut zero = [0; 16];
  zero[0] = 0x80;
  assert_eq!(Dec19::MIN.to_key_bytes(), [0; 16]);
  assert_eq!(Dec19::from_raw(-1).to_key_bytes(), minus_one);
  assert_eq!(Dec19::ZERO.to_key_bytes(), zero);
  assert_eq!(Dec19::MAX.to_key_bytes(), [0xFF; 16]);
  assert_eq!(UDec19::ZERO.to_key_bytes(), [0; 16]);
  assert_eq!(UDec19::MAX.to_key_bytes(), [0xFF; 16]);
  assert_eq!(dec!(1).to_key_bytes(), (D as u128 | 1 << 127).to_be_bytes());
}

#[test]
fn exact_operators() {
  let (a, b) = (dec!(113.725), dec!(0.005));
  assert_eq!(a + b, dec!(113.73));
  assert_eq!(a - b, dec!(113.72));
  assert_eq!(-a, dec!(-113.725));
  assert_eq!(a * 4, dec!(454.9));
  assert_eq!(4 * a, dec!(454.9));
  let mut c = a;
  c += b;
  c -= dec!(0.01);
  c *= 2;
  assert_eq!(c, dec!(227.44));
  let xs = [dec!(1.5), dec!(-0.25), dec!(3)];
  assert_eq!(xs.iter().sum::<Dec19>(), dec!(4.25));
  assert_eq!(xs.into_iter().sum::<Dec19>(), dec!(4.25));
  assert_eq!(udec!(2.5) * 4, udec!(10));
  assert!(dec!(-1) < dec!(0.5) && dec!(0.5) < dec!(1));
  assert_eq!(dec!(-2.5).abs(), dec!(2.5));
  assert!(dec!(-2.5).is_negative() && dec!(2.5).is_positive());
  assert!(dec!(3).is_integer() && !dec!(3.1).is_integer());
  assert!(Dec19::ZERO.is_zero());
}

#[test]
fn checked_and_saturating() {
  let one = Dec19::SMALLEST_STEP;
  assert_eq!(Dec19::MAX.checked_add(one), None);
  assert_eq!(Dec19::MIN.checked_sub(one), None);
  assert_eq!(Dec19::MIN.checked_neg(), None);
  assert_eq!(Dec19::MIN.checked_abs(), None);
  assert_eq!(Dec19::MAX.checked_mul_int(2), None);
  assert_eq!(Dec19::MAX.saturating_add(one), Dec19::MAX);
  assert_eq!(Dec19::MIN.saturating_sub(one), Dec19::MIN);
  assert_eq!(Dec19::MIN.saturating_neg(), Dec19::MAX);
  assert_eq!(Dec19::MIN.saturating_abs(), Dec19::MAX);
  assert_eq!(Dec19::MAX.saturating_mul_int(-2), Dec19::MIN);
  assert_eq!(dec!(1.5).checked_mul_int(-2), Some(dec!(-3)));
  assert_eq!(udec!(1).checked_sub(udec!(2)), None);
  assert_eq!(udec!(1).saturating_sub(udec!(2)), UDec19::ZERO);
}

#[test]
fn operators_panic_on_overflow() {
  let cases: [(&str, fn()); 7] = [
    ("add", || {
      let _ = Dec19::MAX + Dec19::SMALLEST_STEP;
    }),
    ("sub", || {
      let _ = Dec19::MIN - Dec19::SMALLEST_STEP;
    }),
    ("neg", || {
      let _ = -Dec19::MIN;
    }),
    ("mul", || {
      let _ = Dec19::MAX * 2;
    }),
    ("abs", || {
      let _ = Dec19::MIN.abs();
    }),
    ("udec sub", || {
      let _ = udec!(1) - udec!(2);
    }),
    ("sum", || {
      let _: Dec19 = [Dec19::MAX, Dec19::SMALLEST_STEP].iter().sum();
    }),
  ];
  for (name, f) in cases {
    assert!(catch_unwind(f).is_err(), "{name} should panic");
  }
}

#[test]
fn udec19_differences() {
  let (a, b) = (udec!(250.5), udec!(1000));
  assert_eq!(a.abs_diff(b), udec!(749.5));
  assert_eq!(b.abs_diff(a), udec!(749.5));
  assert_eq!(a.signed_sub(b), dec!(-749.5));
  assert_eq!(b.signed_sub(a), dec!(749.5));
  assert_eq!(UDec19::MAX.checked_signed_sub(UDec19::ZERO), None);
  assert_eq!(
    UDec19::ZERO.checked_signed_sub(UDec19::from_raw(1 << 127)),
    Some(Dec19::MIN)
  );
}

#[test]
fn scaled_integers() {
  assert_eq!(Dec19::from_scaled(1_137_250, 4), Some(dec!(113.725)));
  assert_eq!(Dec19::from_scaled(-5, 0), Some(dec!(-5)));
  assert_eq!(Dec19::from_scaled(1, 19), Some(Dec19::SMALLEST_STEP));
  assert_eq!(Dec19::from_scaled(1, 20), None);
  assert_eq!(
    Dec19::from_scaled(i64::MAX, 0).map(|d| d.to_raw()),
    Some(i64::MAX as i128 * D)
  );
  assert_eq!(
    UDec19::from_scaled(u64::MAX, 0),
    Some(UDec19::from(u64::MAX))
  );
  assert_eq!(dec!(113.725).to_scaled(4, Round::HalfEven), Ok(1_137_250));
  assert_eq!(dec!(113.725).to_scaled(2, Round::HalfEven), Ok(11_372));
  assert_eq!(
    dec!(113.725).to_scaled(2, Round::HalfAwayFromZero),
    Ok(11_373)
  );
  assert_eq!(dec!(1).to_scaled(25, Round::HalfEven), Err(OutOfRange));
  assert_eq!(dec!(0).to_scaled(60, Round::HalfEven), Ok(0));
}

#[test]
fn rounding_examples() {
  let tick = dec!(0.005);
  assert_eq!(dec!(113.7268).round_to(tick, Round::Floor), dec!(113.725));
  assert_eq!(dec!(113.7268).round_to(tick, Round::Ceiling), dec!(113.73));
  assert_eq!(dec!(-1.2345).round_to(tick, Round::Floor), dec!(-1.235));
  assert_eq!(dec!(-1.2345).round_to(tick, Round::TowardZero), dec!(-1.23));
  assert_eq!(dec!(25).round_to(dec!(10), Round::HalfEven), dec!(20));
  assert_eq!(dec!(35).round_to(dec!(10), Round::HalfEven), dec!(40));
  assert_eq!(dec!(2.675).round_dp(2, Round::HalfEven), dec!(2.68));
  assert_eq!(dec!(2.665).round_dp(2, Round::HalfEven), dec!(2.66));
  assert_eq!(dec!(-2.5).floor(), dec!(-3));
  assert_eq!(dec!(-2.5).ceil(), dec!(-2));
  assert_eq!(dec!(-2).floor(), dec!(-2));
  assert_eq!(dec!(0.0000000000000000001).ceil(), dec!(1));
  assert!(catch_unwind(|| Dec19::MIN.floor()).is_err());
  assert!(catch_unwind(|| Dec19::MAX.ceil()).is_err());
  assert_eq!(udec!(2.5).floor(), udec!(2));
  assert!(catch_unwind(|| UDec19::MAX.ceil()).is_err());
  assert_eq!(dec!(-2.5).to_int(Round::HalfEven), -2);
  assert_eq!(dec!(-2.5).to_int(Round::Floor), -3);
  assert_eq!(
    dec!(10).div_int(3, Round::HalfEven),
    dec!(3.3333333333333333333)
  );
  assert_eq!(
    dec!(10).div_int(-3, Round::Floor),
    dec!(-3.3333333333333333334)
  );
  assert_eq!(dec!(1250).div_euclid(dec!(100)), 12);
  assert_eq!(dec!(-1250).div_euclid(dec!(100)), -13);
  assert_eq!(dec!(-1250).rem_euclid(dec!(100)), dec!(50));
  // A negative price in a band with a positive tick counts down.
  assert_eq!(dec!(-1.47).div_euclid(dec!(0.05)), -30);
  assert_eq!(dec!(-1.47).rem_euclid(dec!(0.05)), dec!(0.03));
  // A negative divisor: the remainder still isn't negative.
  assert_eq!(dec!(7).div_euclid(dec!(-2)), -3);
  assert_eq!(dec!(7).rem_euclid(dec!(-2)), dec!(1));
  // The one overflow: MIN divided by minus the smallest step.
  let minus_one_step = -Dec19::SMALLEST_STEP;
  assert_eq!(Dec19::MIN.checked_div_euclid(minus_one_step), None);
  assert_eq!(Dec19::MIN.checked_rem_euclid(minus_one_step), None);
  assert!(catch_unwind(|| Dec19::MIN.div_euclid(minus_one_step)).is_err());
  // `%` keeps the sign of the left-hand side, as on integers and f64.
  assert_eq!(dec!(1.47) % dec!(0.05), dec!(0.02));
  assert_eq!(dec!(-1.47) % dec!(0.05), dec!(-0.02));
  assert_eq!(dec!(1.47) % dec!(-0.05), dec!(0.02));
  assert_eq!(dec!(-1.47) % dec!(-0.05), dec!(-0.02));
  assert_eq!(dec!(-1.5) % dec!(0.05), Dec19::ZERO);
  assert_eq!(Dec19::MIN % Dec19::MIN, Dec19::ZERO);
  assert_eq!(Dec19::MIN.checked_rem(minus_one_step), None);
  assert_eq!(dec!(1).checked_rem(Dec19::ZERO), None);
  assert!(catch_unwind(|| Dec19::MIN % minus_one_step).is_err());
  assert!(catch_unwind(|| dec!(1) % Dec19::ZERO).is_err());
  let mut r = dec!(-7);
  r %= dec!(2);
  assert_eq!(r, dec!(-1));
  assert_eq!(dec!(-1.5).unsigned_abs(), udec!(1.5));
  assert_eq!(Dec19::MIN.unsigned_abs().to_raw(), 1 << 127);
  assert_eq!(dec!(-1.5).abs_diff(dec!(2)), udec!(3.5));
  assert_eq!(Dec19::MIN.abs_diff(Dec19::MAX), UDec19::MAX);
  assert_eq!(Dec19::MAX.abs_diff(Dec19::MIN), UDec19::MAX);
  assert_eq!(dec!(1).checked_div_euclid(Dec19::ZERO), None);
  assert_eq!(dec!(1).checked_div_int(0, Round::HalfEven), None);
  assert!(
    catch_unwind(|| dec!(1).round_to(Dec19::ZERO, Round::Floor)).is_err()
  );
  assert!(catch_unwind(|| dec!(1).round_to(dec!(-1), Round::Floor)).is_err());
  assert!(catch_unwind(|| dec!(1).div_int(0, Round::Floor)).is_err());
}

/// Positive steps: tick-like sizes and arbitrary ones.
fn step() -> impl Strategy<Value = i128> {
  prop_oneof![
    (1i128..=1000, 0u32..=20).prop_map(|(m, e)| m * 10i128.pow(e)),
    1i128..=i128::MAX,
  ]
}

proptest! {
  #![proptest_config(config())]

  #[test]
  fn round_to_matches_oracle(raw in value(), step in step()) {
    let x = Dec19::from_raw(raw);
    let s = BigInt::from(step);
    for mode in MODES {
      let expected = dec_fits(&(round_div(&BigInt::from(raw), &s, mode) * &s));
      let got = catch_unwind(|| x.round_to(Dec19::from_raw(step), mode)).ok();
      prop_assert_eq!(got, expected, "{:?}", mode);
    }
  }

  #[test]
  fn udec19_round_to_matches_oracle(raw in uvalue(), step in 1u128..=u128::MAX) {
    let x = UDec19::from_raw(raw);
    let s = BigInt::from(step);
    for mode in MODES {
      let expected = udec_fits(&(round_div(&BigInt::from(raw), &s, mode) * &s));
      let got = catch_unwind(|| x.round_to(UDec19::from_raw(step), mode)).ok();
      prop_assert_eq!(got, expected, "{:?}", mode);
    }
  }

  #[test]
  fn round_dp_and_to_int_match_oracle(raw in value(), places in 0u32..=20) {
    let x = Dec19::from_raw(raw);
    let r = BigInt::from(raw);
    for mode in MODES {
      let expected = if places >= 19 {
        Some(x)
      } else {
        let unit = pow10(19 - places);
        dec_fits(&(round_div(&r, &unit, mode) * &unit))
      };
      let got = catch_unwind(|| x.round_dp(places, mode)).ok();
      prop_assert_eq!(got, expected, "{:?}", mode);
      let int = round_div(&r, &pow10(19), mode);
      prop_assert_eq!(BigInt::from(x.to_int(mode)), int, "{:?}", mode);
    }
  }

  #[test]
  fn floor_and_ceil_match_oracle(raw in value()) {
    let x = Dec19::from_raw(raw);
    // Straight from the floored remainder, not through round_div: floor
    // drops what's above the whole number below; ceil is minus the floor
    // of minus the value.
    let floor = |v: &BigInt| v - v.mod_floor(&BigInt::from(D));
    let r = BigInt::from(raw);
    prop_assert_eq!(catch_unwind(|| x.floor()).ok(), dec_fits(&floor(&r)));
    prop_assert_eq!(catch_unwind(|| x.ceil()).ok(), dec_fits(&-floor(&-&r)));
  }

  #[test]
  fn div_int_matches_oracle(raw in value(), n in any::<i64>()) {
    let x = Dec19::from_raw(raw);
    for mode in MODES {
      let expected = if n == 0 {
        None
      } else {
        // Divide the signed value by a positive divisor, carrying the sign.
        let (num, den) = if n < 0 {
          (-BigInt::from(raw), BigInt::from(n).magnitude().clone().into())
        } else {
          (BigInt::from(raw), BigInt::from(n))
        };
        dec_fits(&round_div(&num, &den, mode))
      };
      prop_assert_eq!(x.checked_div_int(n, mode), expected, "{:?}", mode);
    }
  }

  #[test]
  fn udec19_div_int_matches_oracle(raw in uvalue(), n in any::<u64>()) {
    let x = UDec19::from_raw(raw);
    for mode in MODES {
      let expected = (n != 0)
        .then(|| udec_fits(&round_div(&BigInt::from(raw), &BigInt::from(n), mode)))
        .flatten();
      prop_assert_eq!(x.checked_div_int(n, mode), expected, "{:?}", mode);
    }
  }

  /// Euclidean division, for divisors of either sign: the remainder is
  /// never negative, and the count is whatever makes the two add up.
  #[test]
  fn div_euclid_and_rem_euclid_match_oracle(
    raw in value(),
    step in step(),
    negative_step in any::<bool>(),
  ) {
    let step = if negative_step { -step } else { step };
    let (x, y) = (Dec19::from_raw(raw), Dec19::from_raw(step));
    let (a, b) = (BigInt::from(raw), BigInt::from(step));
    let abs_b = BigInt::from(b.magnitude().clone());
    let r = a.mod_floor(&abs_b); // 0 <= r < |b|
    let q = (&a - &r) / &b; // exact
    let fits = i128::try_from(&q).is_ok();
    prop_assert_eq!(x.checked_div_euclid(y).map(BigInt::from), fits.then(|| q.clone()));
    prop_assert_eq!(
      x.checked_rem_euclid(y).map(|v| BigInt::from(v.to_raw())),
      fits.then(|| r.clone())
    );
    if fits {
      prop_assert_eq!(BigInt::from(x.div_euclid(y)), q);
      prop_assert_eq!(BigInt::from(x.rem_euclid(y).to_raw()), r);
    }
  }

  #[test]
  fn to_scaled_matches_oracle(raw in value(), places in 0u32..=80) {
    let x = Dec19::from_raw(raw);
    let r = BigInt::from(raw);
    for mode in MODES {
      let exact = if places <= 19 {
        round_div(&r, &pow10(19 - places), mode)
      } else {
        &r * pow10(places - 19)
      };
      prop_assert_eq!(
        x.to_scaled(places, mode).ok(),
        i64::try_from(&exact).ok(),
        "{:?}",
        mode
      );
    }
  }

  #[test]
  fn key_bytes_sort_like_values(a in value(), b in value()) {
    let (x, y) = (Dec19::from_raw(a), Dec19::from_raw(b));
    prop_assert_eq!(x.to_key_bytes().cmp(&y.to_key_bytes()), x.cmp(&y));
    prop_assert_eq!(Dec19::from_key_bytes(x.to_key_bytes()), x);
  }

  #[test]
  fn udec19_key_bytes_sort_like_values(a in uvalue(), b in uvalue()) {
    let (x, y) = (UDec19::from_raw(a), UDec19::from_raw(b));
    prop_assert_eq!(x.to_key_bytes().cmp(&y.to_key_bytes()), x.cmp(&y));
    prop_assert_eq!(UDec19::from_key_bytes(x.to_key_bytes()), x);
  }

  #[test]
  fn checked_ops_match_oracle(a in value(), b in value(), n in any::<i64>()) {
    let (x, y) = (Dec19::from_raw(a), Dec19::from_raw(b));
    let (ba, bb) = (BigInt::from(a), BigInt::from(b));
    prop_assert_eq!(x.checked_add(y), dec_fits(&(&ba + &bb)));
    prop_assert_eq!(x.checked_sub(y), dec_fits(&(&ba - &bb)));
    prop_assert_eq!(x.checked_mul_int(n), dec_fits(&(&ba * n)));
    let clamp = |v: BigInt| {
      dec_fits(&v).unwrap_or(if v.sign() == num_bigint::Sign::Minus {
        Dec19::MIN
      } else {
        Dec19::MAX
      })
    };
    prop_assert_eq!(x.saturating_add(y), clamp(&ba + &bb));
    prop_assert_eq!(x.saturating_sub(y), clamp(&ba - &bb));
    prop_assert_eq!(x.saturating_mul_int(n), clamp(&ba * n));
  }
}
