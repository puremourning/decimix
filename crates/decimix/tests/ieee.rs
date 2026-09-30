//! The f64 boundary, checked against std's float parsing and formatting and
//! a big-integer oracle.

mod common;

use common::{D, MODES, config, pow10, round_div, uvalue, value};
use decimix::{Dec19, FromF64Error, Round, UDec19, dec, udec};
use num_bigint::BigInt;
use proptest::prelude::*;

/// What `from_f64_lossy(x, step, mode)` should give, as a stored value:
/// std's shortest decimal for `x`, rounded once to a multiple of `step`,
/// with exact big-integer arithmetic. `None` means out of range for the
/// stored-value bounds `(min, max)`.
fn from_f64_oracle(
  x: f64,
  step: u128,
  mode: Round,
  (min, max): (&BigInt, &BigInt),
) -> Result<BigInt, FromF64Error> {
  if !x.is_finite() {
    return Err(FromF64Error::NotFinite);
  }
  let text = format!("{}", x.abs());
  let (int, frac) = text.split_once('.').unwrap_or((&text, ""));
  let digits: BigInt = format!("{int}{frac}").parse().unwrap();
  let digits = if x < 0.0 { -digits } else { digits };
  // value = digits / 10^places; in stored steps, times 10^19; in multiples
  // of `step`, divided by step. Round that once.
  let num = digits * pow10(19);
  let den = pow10(frac.len() as u32) * BigInt::from(step);
  let v = round_div(&num, &den, mode) * BigInt::from(step);
  if &v < min || &v > max {
    return Err(FromF64Error::OutOfRange);
  }
  Ok(v)
}

fn dec_bounds() -> (BigInt, BigInt) {
  (BigInt::from(i128::MIN), BigInt::from(i128::MAX))
}

fn udec_bounds() -> (BigInt, BigInt) {
  (BigInt::ZERO, BigInt::from(u128::MAX))
}

/// Doubles as they arrive from feeds and APIs, plus the awkward ones.
fn double() -> impl Strategy<Value = f64> {
  prop_oneof![
    // Prices: a few significant digits at various scales.
    (any::<i32>(), 0u32..=8).prop_map(|(m, e)| m as f64 / 10f64.powi(e as i32)),
    // Arithmetic noise, like 0.1 + 0.2.
    (1u32..1000, 1u32..1000)
      .prop_map(|(a, b)| a as f64 / 10.0 + b as f64 / 100.0),
    // Anything at all, including NaN, infinities and subnormals.
    any::<f64>(),
    // Tiny: around and below the smallest step.
    (1u64..1_000_000, 15i32..=30).prop_map(|(m, e)| m as f64 * 10f64.powi(-e)),
    // Near the edges of the range.
    (-3.5e19f64..3.5e19f64),
    prop::sample::select(vec![
      0.0,
      -0.0,
      1e-20,
      -1e-20,
      5e-20,
      1e20,
      -1e20,
      1.7014118346046923e19,
      3.4028236692093846e19,
      f64::MIN_POSITIVE,
      5e-324,
      f64::MAX,
      f64::NAN,
      f64::INFINITY,
      f64::NEG_INFINITY,
    ]),
  ]
}

/// Steps: the smallest, powers of ten, tick sizes, and arbitrary ones.
fn step() -> impl Strategy<Value = u128> {
  prop_oneof![
    Just(1u128),
    (0u32..=25).prop_map(|k| 10u128.pow(k)),
    (1u128..=100, 12u32..=20).prop_map(|(m, e)| m * 10u128.pow(e)),
    1u128..=i128::MAX as u128,
  ]
}

/// The exact value of a finite double, as a numerator and a positive
/// denominator: every double is an integer times a power of two.
fn exact(f: f64) -> (BigInt, BigInt) {
  let bits = f.to_bits();
  let exp = ((bits >> 52) & 0x7ff) as i32;
  let frac = bits & ((1 << 52) - 1);
  // Subnormals have no hidden bit and the smallest exponent.
  let (m, e) = if exp == 0 {
    (frac, -1074)
  } else {
    (frac | (1 << 52), exp - 1075)
  };
  let m = if bits >> 63 == 1 {
    -BigInt::from(m)
  } else {
    BigInt::from(m)
  };
  if e >= 0 {
    (m << e as u32, BigInt::from(1))
  } else {
    (m, BigInt::from(1) << (-e) as u32)
  }
}

/// How far double `d` is from the stored value `raw` (× 10^-19), as a
/// fraction (numerator, denominator), both non-negative.
fn distance(d: f64, raw: &BigInt) -> (BigInt, BigInt) {
  let (n, den) = exact(d);
  let diff = n * pow10(19) - raw * &den;
  (BigInt::from(diff.magnitude().clone()), den * pow10(19))
}

/// Checks that `d` is the double nearest to `raw` × 10^-19, ties to even,
/// with exact arithmetic and no float parsing at all.
fn is_nearest(d: f64, raw: &BigInt) -> bool {
  let (dn, dd) = distance(d, raw);
  [d.next_up(), d.next_down()].iter().all(|&other| {
    let (on, od) = distance(other, raw);
    let (mine, theirs) = (&dn * &od, &on * &dd);
    // Strictly closer, or an exact tie with d's last bit even.
    mine < theirs || (mine == theirs && d.to_bits() & 1 == 0)
  })
}

proptest! {
  #![proptest_config(config())]

  /// The same promise checked from first principles: exact arithmetic on
  /// the double's binary value and its neighbours, independent of std's
  /// parser (which the implementation's slow path uses).
  #[test]
  fn to_f64_is_the_nearest_double(raw in value(), uraw in uvalue()) {
    let d = Dec19::from_raw(raw).to_f64_lossy();
    prop_assert!(is_nearest(d, &BigInt::from(raw)), "{} -> {:?}", raw, d);
    let d = UDec19::from_raw(uraw).to_f64_lossy();
    prop_assert!(is_nearest(d, &BigInt::from(uraw)), "{} -> {:?}", uraw, d);
  }

  /// The nearest double, bit for bit what std gives when parsing the text.
  #[test]
  fn to_f64_matches_std_parse(raw in value()) {
    let x = Dec19::from_raw(raw);
    let expected: f64 = x.to_string().parse().unwrap();
    prop_assert_eq!(x.to_f64_lossy().to_bits(), expected.to_bits());
  }

  #[test]
  fn udec19_to_f64_matches_std_parse(raw in uvalue()) {
    let x = UDec19::from_raw(raw);
    let expected: f64 = x.to_string().parse().unwrap();
    prop_assert_eq!(x.to_f64_lossy().to_bits(), expected.to_bits());
  }

  #[test]
  fn from_f64_matches_oracle(x in double(), step in step()) {
    let (dmin, dmax) = dec_bounds();
    let (umin, umax) = udec_bounds();
    for mode in MODES {
      if step <= i128::MAX as u128 {
        let got = Dec19::from_f64_lossy(x, Dec19::from_raw(step as i128), mode);
        let want = from_f64_oracle(x, step, mode, (&dmin, &dmax))
          .map(|v| Dec19::from_raw(i128::try_from(v).unwrap()));
        prop_assert_eq!(got, want, "{:?}", mode);
      }
      let got = UDec19::from_f64_lossy(x, UDec19::from_raw(step), mode);
      let want = from_f64_oracle(x, step, mode, (&umin, &umax))
        .map(|v| UDec19::from_raw(u128::try_from(v).unwrap()));
      prop_assert_eq!(got, want, "{:?}", mode);
    }
  }

  /// Any value with at most 15 significant digits survives a trip through
  /// f64 (a double holds 15 decimal digits reliably).
  #[test]
  fn fifteen_digits_round_trip(n in -999_999_999_999_999i64..=999_999_999_999_999, e in -19i32..=4) {
    let raw = BigInt::from(n) * pow10((19 + e) as u32);
    let x = Dec19::from_raw(i128::try_from(raw).unwrap());
    let back = Dec19::from_f64_lossy(x.to_f64_lossy(), Dec19::SMALLEST_STEP, Round::HalfEven);
    prop_assert_eq!(back, Ok(x));
  }
}

#[test]
fn examples() {
  assert_eq!(dec!(113.725).to_f64_lossy(), 113.725);
  assert_eq!(dec!(-0.1).to_f64_lossy(), -0.1);
  assert_eq!(Dec19::ZERO.to_f64_lossy().to_bits(), 0.0f64.to_bits());
  assert_eq!(dec!(1).to_f64_lossy(), 1.0);
  assert_eq!(Dec19::MAX.to_f64_lossy(), 1.7014118346046923e19);
  assert_eq!(UDec19::MAX.to_f64_lossy(), 3.4028236692093846e19);

  let noisy = 0.1 + 0.2;
  let fine = Dec19::from_f64_lossy(noisy, dec!(0.000001), Round::HalfEven);
  assert_eq!(fine, Ok(dec!(0.3)));
  let all = Dec19::from_f64_lossy(noisy, Dec19::SMALLEST_STEP, Round::HalfEven);
  assert_eq!(all, Ok(dec!(0.30000000000000004)));
  assert_eq!(
    Dec19::from_f64_lossy(113.7249999999, dec!(0.005), Round::HalfEven),
    Ok(dec!(113.725))
  );
  assert_eq!(
    Dec19::from_f64_lossy(-0.0, Dec19::SMALLEST_STEP, Round::Floor),
    Ok(Dec19::ZERO)
  );
  for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
    assert_eq!(
      Dec19::from_f64_lossy(bad, Dec19::ONE, Round::HalfEven),
      Err(FromF64Error::NotFinite)
    );
  }
  assert_eq!(
    Dec19::from_f64_lossy(2e19, Dec19::ONE, Round::HalfEven),
    Err(FromF64Error::OutOfRange)
  );
  assert_eq!(
    UDec19::from_f64_lossy(2e19, UDec19::ONE, Round::HalfEven),
    Ok(udec!(20000000000000000000))
  );
  assert_eq!(
    UDec19::from_f64_lossy(-1.0, UDec19::ONE, Round::HalfEven),
    Err(FromF64Error::OutOfRange)
  );
  // A tiny negative double rounds to zero, which an unsigned type can hold.
  assert_eq!(
    UDec19::from_f64_lossy(-1e-25, UDec19::SMALLEST_STEP, Round::Ceiling),
    Ok(UDec19::ZERO)
  );
  // Tiny doubles, directed modes: a single step away from zero.
  assert_eq!(
    Dec19::from_f64_lossy(1e-300, Dec19::SMALLEST_STEP, Round::Ceiling),
    Ok(Dec19::SMALLEST_STEP)
  );
  assert_eq!(
    Dec19::from_f64_lossy(-5e-324, dec!(0.01), Round::Floor),
    Ok(dec!(-0.01))
  );
  assert!(
    std::panic::catch_unwind(|| {
      Dec19::from_f64_lossy(1.0, Dec19::ZERO, Round::HalfEven)
    })
    .is_err()
  );
  assert_eq!(D, Dec19::ONE.to_raw());
}

/// Rounding the double's decimal once, not first to 19 places and then to
/// the step. 1.46e-18 is 14.6 smallest steps: to a step of 10 smallest steps
/// it rounds to 10. Rounding to 19 places first would make it 15, an exact
/// tie, which half-even and half-away-from-zero would take up to 20.
#[test]
fn no_double_rounding() {
  let step = Dec19::from_raw(10);
  for mode in [Round::HalfEven, Round::HalfAwayFromZero] {
    assert_eq!(
      Dec19::from_f64_lossy(1.46e-18, step, mode),
      Ok(Dec19::from_raw(10)),
      "{mode:?}"
    );
  }
}
