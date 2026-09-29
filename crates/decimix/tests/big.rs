//! Conversions to and from `BigDecimal`, checked against a big-integer
//! oracle. Needs `--features bigdecimal`.
#![cfg(feature = "bigdecimal")]

mod common;

use std::time::{Duration, Instant};

use bigdecimal::BigDecimal;
use common::{MODES, config, pow10, round_div, uvalue, value};
use decimix::{Dec19, OutOfRange, Round, TryFromBigError, UDec19, dec, udec};
use num_bigint::BigInt;
use proptest::prelude::*;

/// `digits × 10^-scale` in stored steps (× 10^19): the exact numerator and
/// denominator.
fn exact_steps(digits: &BigInt, scale: i64) -> (BigInt, BigInt) {
  let shift = 19 - scale;
  if shift >= 0 {
    (digits * pow10(shift as u32), BigInt::from(1))
  } else {
    (digits.clone(), pow10((-shift) as u32))
  }
}

fn digits() -> impl Strategy<Value = BigInt> {
  prop_oneof![
    any::<i128>().prop_map(BigInt::from),
    any::<i64>().prop_map(BigInt::from),
    (any::<i128>(), any::<u128>())
      .prop_map(|(h, l)| (BigInt::from(h) << 128u32) + BigInt::from(l)),
  ]
}

proptest! {
  #![proptest_config(config())]

  #[test]
  fn round_trips(raw in value(), uraw in uvalue()) {
    let x = Dec19::from_raw(raw);
    let b = x.to_big();
    prop_assert_eq!(&b, &x.to_string().parse::<BigDecimal>().unwrap());
    prop_assert_eq!(Dec19::try_from(&b), Ok(x));
    let u = UDec19::from_raw(uraw);
    prop_assert_eq!(UDec19::try_from(&u.to_big()), Ok(u));
    for mode in MODES {
      prop_assert_eq!(Dec19::from_big(&b, mode), Ok(x));
      prop_assert_eq!(UDec19::from_big(&u.to_big(), mode), Ok(u));
    }
  }

  #[test]
  fn from_big_matches_oracle(d in digits(), scale in -45i64..=70) {
    let b = BigDecimal::new(d.clone(), scale);
    let (num, den) = exact_steps(&d, scale);
    for mode in MODES {
      let rounded = round_div(&num, &den, mode);
      let want = i128::try_from(&rounded).map(Dec19::from_raw).map_err(|_| OutOfRange);
      prop_assert_eq!(Dec19::from_big(&b, mode), want, "{:?}", mode);
      let want = u128::try_from(&rounded).map(UDec19::from_raw).map_err(|_| OutOfRange);
      prop_assert_eq!(UDec19::from_big(&b, mode), want, "{:?}", mode);
    }
    // Exact conversion: range first, then exactness.
    let truncated = round_div(&num, &den, Round::TowardZero);
    let exact = &truncated * &den == num;
    let want = match i128::try_from(&truncated) {
      Err(_) => Err(TryFromBigError::OutOfRange),
      Ok(_) if !exact => Err(TryFromBigError::Inexact),
      Ok(v) => Ok(Dec19::from_raw(v)),
    };
    prop_assert_eq!(Dec19::try_from(&b), want);
  }
}

#[test]
fn examples() {
  let b: BigDecimal = dec!(113.725).into();
  assert_eq!(b.to_string(), "113.7250000000000000000");
  let third = BigDecimal::from(1) / BigDecimal::from(3);
  assert_eq!(Dec19::try_from(&third), Err(TryFromBigError::Inexact));
  assert_eq!(
    Dec19::from_big(&third, Round::HalfEven),
    Ok(dec!(0.3333333333333333333))
  );
  assert_eq!(
    Dec19::from_big(&third, Round::Ceiling),
    Ok(dec!(0.3333333333333333334))
  );
  let big: BigDecimal = "1e25".parse().unwrap();
  assert_eq!(Dec19::try_from(&big), Err(TryFromBigError::OutOfRange));
  assert_eq!(Dec19::from_big(&big, Round::HalfEven), Err(OutOfRange));
  let neg: BigDecimal = "-2".parse().unwrap();
  assert_eq!(UDec19::try_from(&neg), Err(TryFromBigError::OutOfRange));
  assert_eq!(UDec19::try_from(&BigDecimal::from(7)), Ok(udec!(7)));
}

/// Absurd exponents come back quickly, without building huge numbers.
#[test]
fn extreme_exponents_are_cheap() {
  let start = Instant::now();
  let tiny = BigDecimal::new(BigInt::from(1), 1_000_000); // 1e-1000000
  let huge = BigDecimal::new(BigInt::from(1), -1_000_000); // 1e1000000
  let neg_tiny = BigDecimal::new(BigInt::from(-7), 1_000_000);
  for mode in MODES {
    assert!(Dec19::from_big(&huge, mode).is_err());
    let expected = match mode {
      Round::Ceiling | Round::AwayFromZero => Dec19::SMALLEST_STEP,
      _ => Dec19::ZERO,
    };
    assert_eq!(Dec19::from_big(&tiny, mode), Ok(expected), "{mode:?}");
    let expected = match mode {
      Round::Floor | Round::AwayFromZero => -Dec19::SMALLEST_STEP,
      _ => Dec19::ZERO,
    };
    assert_eq!(Dec19::from_big(&neg_tiny, mode), Ok(expected), "{mode:?}");
  }
  assert_eq!(Dec19::try_from(&tiny), Err(TryFromBigError::Inexact));
  assert_eq!(Dec19::try_from(&huge), Err(TryFromBigError::OutOfRange));
  // Miri runs ~1000x slower, so only time it natively.
  if !cfg!(miri) {
    assert!(
      start.elapsed() < Duration::from_secs(1),
      "took {:?}",
      start.elapsed()
    );
  }
}
