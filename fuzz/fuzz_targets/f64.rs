//! Arbitrary doubles and steps into from_f64_lossy, against an exact
//! oracle; and to_f64_lossy against std's parser.
#![no_main]

use decimix::{Dec19, FromF64Error};
use decimix_fuzz::{mode, pow10, round_div};
use libfuzzer_sys::fuzz_target;
use num_bigint::BigInt;

fuzz_target!(|input: (u64, i128, i128, u8)| {
  let (bits, step, raw, m) = input;
  let mode = mode(m);

  // to_f64_lossy: bit for bit what std's parser gives for the text.
  let v = Dec19::from_raw(raw);
  let expected: f64 = v.to_string().parse().unwrap();
  assert_eq!(v.to_f64_lossy().to_bits(), expected.to_bits());

  // from_f64_lossy: the shortest decimal of x, rounded once to the step.
  let x = f64::from_bits(bits);
  let step = step.unsigned_abs().max(1).min(i128::MAX as u128) as i128;
  let got = Dec19::from_f64_lossy(x, Dec19::from_raw(step), mode);
  let expected = if !x.is_finite() {
    Err(FromF64Error::NotFinite)
  } else {
    let text = format!("{}", x.abs());
    let (int, frac) = text.split_once('.').unwrap_or((&text, ""));
    let digits: BigInt = format!("{int}{frac}").parse().unwrap();
    let digits = if x < 0.0 { -digits } else { digits };
    let num = digits * pow10(19);
    let den = pow10(frac.len() as u32) * BigInt::from(step);
    let v = round_div(&num, &den, mode) * BigInt::from(step);
    i128::try_from(&v)
      .map(Dec19::from_raw)
      .map_err(|_| FromF64Error::OutOfRange)
  };
  assert_eq!(got, expected, "{x:?} step {step} {mode:?}");
});
