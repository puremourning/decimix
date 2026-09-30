//! Multiplication and division of arbitrary values against an exact oracle.
#![no_main]

use decimix::Dec19;
use decimix_fuzz::{mode, pow10, round_div};
use libfuzzer_sys::fuzz_target;
use num_bigint::BigInt;

fuzz_target!(|input: (i128, i128, u8)| {
  let (a, b, m) = input;
  let mode = mode(m);
  let (x, y) = (Dec19::from_raw(a), Dec19::from_raw(b));
  let d = pow10(19);

  let product = round_div(&(BigInt::from(a) * BigInt::from(b)), &d, mode);
  let expected = i128::try_from(&product).ok().map(Dec19::from_raw);
  assert_eq!(x.checked_mul(y, mode), expected, "{a} * {b} {mode:?}");

  let expected = (b != 0)
    .then(|| {
      let q = round_div(&(BigInt::from(a) * &d), &BigInt::from(b), mode);
      i128::try_from(&q).ok().map(Dec19::from_raw)
    })
    .flatten();
  assert_eq!(x.checked_div(y, mode), expected, "{a} / {b} {mode:?}");
});
