//! `UDec19`-specific behaviour, and the shared API on the unsigned type,
//! checked against big-integer oracles.

mod common;

use std::panic::catch_unwind;

use common::{D, MODES, config, pow10, round_div, uvalue, value};
use decimix::{Dec19, OutOfRange, Round, UDec19, dec, udec};
use num_bigint::BigInt;
use num_integer::Integer;
use proptest::prelude::*;

fn ubig(v: u128) -> BigInt {
  BigInt::from(v)
}

fn udec_fits(v: &BigInt) -> Option<UDec19> {
  u128::try_from(v).ok().map(UDec19::from_raw)
}

fn dec_fits(v: &BigInt) -> Option<Dec19> {
  i128::try_from(v).ok().map(Dec19::from_raw)
}

/// Pairs of rounding modes that must agree when nothing is negative.
const SAME_WITHOUT_NEGATIVES: [(Round, Round); 2] = [
  (Round::Floor, Round::TowardZero),
  (Round::Ceiling, Round::AwayFromZero),
];

proptest! {
  #![proptest_config(config())]

  #[test]
  fn checked_and_saturating_match_oracle(
    a in uvalue(),
    b in uvalue(),
    n in any::<u64>(),
  ) {
    let (x, y) = (UDec19::from_raw(a), UDec19::from_raw(b));
    let (ba, bb) = (ubig(a), ubig(b));
    let clamp = |v: BigInt| {
      udec_fits(&v).unwrap_or(if v.sign() == num_bigint::Sign::Minus {
        UDec19::ZERO
      } else {
        UDec19::MAX
      })
    };
    prop_assert_eq!(x.checked_add(y), udec_fits(&(&ba + &bb)));
    prop_assert_eq!(x.checked_sub(y), udec_fits(&(&ba - &bb)));
    prop_assert_eq!(x.checked_mul_int(n), udec_fits(&(&ba * n)));
    prop_assert_eq!(x.saturating_add(y), clamp(&ba + &bb));
    prop_assert_eq!(x.saturating_sub(y), clamp(&ba - &bb));
    prop_assert_eq!(x.saturating_mul_int(n), clamp(&ba * n));
  }

  #[test]
  fn differences_match_oracle(a in uvalue(), b in uvalue()) {
    let (x, y) = (UDec19::from_raw(a), UDec19::from_raw(b));
    let diff = ubig(a) - ubig(b);
    prop_assert_eq!(x.abs_diff(y), udec_fits(&diff.magnitude().clone().into()).unwrap());
    prop_assert_eq!(x.checked_signed_sub(y), dec_fits(&diff));
    match dec_fits(&diff) {
      Some(d) => prop_assert_eq!(x.signed_sub(y), d),
      None => prop_assert!(catch_unwind(|| x.signed_sub(y)).is_err()),
    }
  }

  #[test]
  fn conversions_match_oracle(u in uvalue(), s in value()) {
    prop_assert_eq!(
      Dec19::try_from(UDec19::from_raw(u)).ok(),
      dec_fits(&ubig(u))
    );
    prop_assert_eq!(
      UDec19::try_from(Dec19::from_raw(s)).ok(),
      udec_fits(&BigInt::from(s))
    );
  }

  #[test]
  fn integer_conversions_match_oracle(n in any::<i128>(), m in any::<u64>()) {
    let expected = udec_fits(&(BigInt::from(n) * BigInt::from(D)));
    prop_assert_eq!(UDec19::try_from(n).ok(), expected);
    prop_assert_eq!(UDec19::from(m).to_raw(), m as u128 * D as u128);
  }

  #[test]
  fn div_floor_and_rem_euclid_match_oracle(a in uvalue(), b in 1u128..=u128::MAX) {
    let (x, y) = (UDec19::from_raw(a), UDec19::from_raw(b));
    let (q, r) = ubig(a).div_mod_floor(&ubig(b));
    prop_assert_eq!(ubig(x.div_floor(y)), q);
    prop_assert_eq!(ubig(x.rem_euclid(y).to_raw()), r);
  }

  #[test]
  fn round_dp_and_to_int_match_oracle(raw in uvalue(), places in 0u32..=20) {
    let x = UDec19::from_raw(raw);
    for mode in MODES {
      let expected = if places >= 19 {
        Some(x)
      } else {
        let unit = pow10(19 - places);
        udec_fits(&(round_div(&ubig(raw), &unit, mode) * &unit))
      };
      prop_assert_eq!(catch_unwind(|| x.round_dp(places, mode)).ok(), expected);
      prop_assert_eq!(ubig(x.to_int(mode)), round_div(&ubig(raw), &pow10(19), mode));
    }
  }

  #[test]
  fn scaled_integers_match_oracle(raw in uvalue(), places in 0u32..=45, n in any::<u64>()) {
    let x = UDec19::from_raw(raw);
    for mode in MODES {
      let exact = if places <= 19 {
        round_div(&ubig(raw), &pow10(19 - places), mode)
      } else {
        ubig(raw) * pow10(places - 19)
      };
      prop_assert_eq!(x.to_scaled(places, mode).ok(), u64::try_from(&exact).ok());
    }
    let expected = (places <= 19).then(|| {
      UDec19::from_raw(n as u128 * 10u128.pow(19 - places))
    });
    prop_assert_eq!(UDec19::from_scaled(n, places), expected);
  }

  /// With no negative values, rounding down and rounding toward zero are the
  /// same thing, as are rounding up and rounding away from zero.
  #[test]
  fn directed_modes_pair_up(a in uvalue(), b in uvalue(), n in 1u64..=u64::MAX) {
    let (x, y) = (UDec19::from_raw(a), UDec19::from_raw(b));
    for (m1, m2) in SAME_WITHOUT_NEGATIVES {
      prop_assert_eq!(x.checked_mul(y, m1), x.checked_mul(y, m2));
      prop_assert_eq!(x.checked_div(y, m1), x.checked_div(y, m2));
      prop_assert_eq!(x.checked_div_int(n, m1), x.checked_div_int(n, m2));
      prop_assert_eq!(x.to_int(m1), x.to_int(m2));
      prop_assert_eq!(
        catch_unwind(|| x.round_dp(3, m1)).ok(),
        catch_unwind(|| x.round_dp(3, m2)).ok()
      );
    }
  }
}

#[test]
fn subtraction_below_zero_panics_with_a_clear_message() {
  let err = catch_unwind(|| udec!(1) - udec!(2)).unwrap_err();
  let msg = err
    .downcast_ref::<String>()
    .map(String::as_str)
    .or_else(|| err.downcast_ref::<&str>().copied())
    .unwrap_or_default();
  assert_eq!(msg, "UDec19 subtraction would be negative");
  let mut q = udec!(1);
  assert!(
    catch_unwind(move || {
      q -= udec!(1.5);
      q
    })
    .is_err()
  );
}

#[test]
fn operators_and_sums() {
  assert_eq!(udec!(2.5) * 4, udec!(10));
  assert_eq!(4 * udec!(2.5), udec!(10));
  assert_eq!([udec!(1.5), udec!(2)].iter().sum::<UDec19>(), udec!(3.5));
  assert!(catch_unwind(|| UDec19::MAX * 2).is_err());
  assert!(
    catch_unwind(|| [UDec19::MAX, udec!(1)].iter().sum::<UDec19>()).is_err()
  );
  assert_eq!(udec!(0).checked_sub(udec!(0)), Some(UDec19::ZERO));
}

#[test]
fn range_beyond_dec19() {
  // Above Dec19::MAX but fine as a UDec19: about 3.4e19 whole units.
  let big = udec!(30000000000000000000);
  assert_eq!(Dec19::try_from(big), Err(OutOfRange));
  assert_eq!(big.to_string(), "30000000000000000000");
  assert_eq!(big.signed_sub(udec!(29999999999999999999)), dec!(1));
  assert_eq!(
    UDec19::MAX.to_int(Round::TowardZero),
    34_028_236_692_093_846_346
  );
  // "-0" is zero, not a negative number.
  assert_eq!("-0".parse::<UDec19>(), Ok(UDec19::ZERO));
  assert_eq!(
    UDec19::parse_round("-0.00000000000000000001", Round::Ceiling),
    Ok(UDec19::ZERO)
  );
  assert!(
    UDec19::parse_round("-0.00000000000000000001", Round::Floor).is_err()
  );
}
