//! Text in and out, checked against big-integer oracles.

mod common;

use common::{
  D,
  MODES,
  config,
  fixed_text,
  pow10,
  reference_text,
  round_div,
  uvalue,
  value,
};
use decimix::{BufferTooSmall, Dec19, ParseError, Round, UDec19, dec, udec};
use num_bigint::BigInt;
use proptest::prelude::*;

/// What parsing `text` should give, as a stored value, worked out with big
/// integers: the grammar by hand, then exact arithmetic.
///
/// `range` is the type's (min, max) stored value. Errors in the crate's
/// documented order: Invalid, then OutOfRange, then TooPrecise.
fn parse_oracle(
  text: &[u8],
  mode: Option<Round>,
  range: (&BigInt, &BigInt),
) -> Result<BigInt, ParseError> {
  // Grammar: [+-]? digits? ('.' digits?)? with at least one digit.
  let (negative, rest) = match text.first() {
    Some(b'-') => (true, &text[1..]),
    Some(b'+') => (false, &text[1..]),
    _ => (false, text),
  };
  let (int, frac) = match rest.iter().position(|&c| c == b'.') {
    Some(p) => (&rest[..p], &rest[p + 1..]),
    None => (rest, &rest[rest.len()..]),
  };
  let all_digits = |s: &[u8]| s.iter().all(u8::is_ascii_digit);
  if !all_digits(int) || !all_digits(frac) || int.len() + frac.len() == 0 {
    return Err(ParseError::Invalid);
  }

  // The exact value as digits / 10^frac_len, in stored units.
  let digits: String = int.iter().chain(frac).map(|&c| c as char).collect();
  let n = BigInt::parse_bytes(format!("0{digits}").as_bytes(), 10).unwrap();
  let n = if negative { -n } else { n };
  let num = n * pow10(19);
  let den = pow10(frac.len() as u32);

  let u128_max = BigInt::from(u128::MAX);
  let truncated = round_div(&num, &den, Round::TowardZero);
  if truncated.magnitude() > u128_max.magnitude() {
    return Err(ParseError::OutOfRange);
  }
  let value = match mode {
    Some(mode) => round_div(&num, &den, mode),
    None => truncated.clone(),
  };
  if value.magnitude() > u128_max.magnitude()
    || &value < range.0
    || &value > range.1
  {
    return Err(ParseError::OutOfRange);
  }
  if mode.is_none() && &truncated * &den != num {
    return Err(ParseError::TooPrecise);
  }
  Ok(value)
}

fn dec_range() -> (BigInt, BigInt) {
  (BigInt::from(i128::MIN), BigInt::from(i128::MAX))
}

fn udec_range() -> (BigInt, BigInt) {
  (BigInt::ZERO, BigInt::from(u128::MAX))
}

/// Decimal text with the awkward parts made likely: leading zeros, long
/// runs, exact halves beyond the 19th place, trailing zeros, and bare points.
fn decimal_text() -> impl Strategy<Value = String> {
  let sign = prop_oneof![4 => Just(""), 2 => Just("-"), 1 => Just("+")];
  let int = prop_oneof![
    "[0-9]{0,4}",
    "0{0,3}[1-9][0-9]{0,21}",
    "9{15,21}",
    "1[0-9]{19}",
    "3[0-9]{19}",
  ];
  let frac = prop_oneof![
    Just(None),
    Just(Some(String::new())),
    "[0-9]{1,19}".prop_map(Some),
    "[0-9]{19}50*".prop_map(Some),
    "[0-9]{19}[0-9]{1,6}".prop_map(Some),
    "[0-9]{1,10}0{10,20}".prop_map(Some),
  ];
  (sign, int, frac).prop_map(|(s, i, f)| match f {
    Some(f) => format!("{s}{i}.{f}"),
    None => format!("{s}{i}"),
  })
}

/// Arbitrary bytes, mostly digits, with the grammar's punctuation and bytes
/// that stress the SWAR checks.
fn text_bytes() -> impl Strategy<Value = Vec<u8>> {
  let byte = prop_oneof![
    8 => b'0'..=b'9',
    2 => Just(b'.'),
    1 => Just(b'-'),
    1 => Just(b'+'),
    1 => prop::sample::select(vec![b'/', b':', b' ', b'e', b'_', 0, 0x80, 0xFF]),
  ];
  prop::collection::vec(byte, 0..48)
}

fn check_parse(text: &[u8]) -> Result<(), TestCaseError> {
  let (dmin, dmax) = dec_range();
  let (umin, umax) = udec_range();
  let as_dec = |r: BigInt| Dec19::from_raw(i128::try_from(r).unwrap());
  let as_udec = |r: BigInt| UDec19::from_raw(u128::try_from(r).unwrap());

  prop_assert_eq!(
    Dec19::from_ascii(text),
    parse_oracle(text, None, (&dmin, &dmax)).map(as_dec)
  );
  prop_assert_eq!(
    UDec19::from_ascii(text),
    parse_oracle(text, None, (&umin, &umax)).map(as_udec)
  );
  for mode in MODES {
    prop_assert_eq!(
      Dec19::from_ascii_round(text, mode),
      parse_oracle(text, Some(mode), (&dmin, &dmax)).map(as_dec),
      "{:?}",
      mode
    );
    prop_assert_eq!(
      UDec19::from_ascii_round(text, mode),
      parse_oracle(text, Some(mode), (&umin, &umax)).map(as_udec),
      "{:?}",
      mode
    );
  }
  Ok(())
}

proptest! {
  #![proptest_config(config())]

  #[test]
  fn dec19_text_round_trips(raw in value()) {
    let x = Dec19::from_raw(raw);
    let expected = reference_text(&BigInt::from(raw));
    prop_assert_eq!(x.to_string(), expected.clone());
    let ascii = x.to_ascii();
    prop_assert_eq!(ascii.as_str(), expected.as_str());
    let mut buf = [0u8; Dec19::MAX_ASCII_LEN];
    let n = x.write_ascii(&mut buf).unwrap();
    prop_assert_eq!(&buf[..n], expected.as_bytes());
    prop_assert_eq!(Dec19::from_ascii(expected.as_bytes()), Ok(x));
    prop_assert_eq!(expected.parse::<Dec19>(), Ok(x));
  }

  #[test]
  fn udec19_text_round_trips(raw in uvalue()) {
    let x = UDec19::from_raw(raw);
    let expected = reference_text(&BigInt::from(raw));
    prop_assert_eq!(x.to_string(), expected.clone());
    let mut buf = [0u8; UDec19::MAX_ASCII_LEN];
    let n = x.write_ascii(&mut buf).unwrap();
    prop_assert_eq!(&buf[..n], expected.as_bytes());
    prop_assert_eq!(UDec19::from_ascii(expected.as_bytes()), Ok(x));
  }

  #[test]
  fn fixed_places_match_oracle(raw in value(), places in 0u32..=25) {
    let x = Dec19::from_raw(raw);
    let raw = BigInt::from(raw);
    let mut buf = [0u8; 64];
    for mode in MODES {
      let expected = if places >= 19 {
        fixed_text(&(&raw * pow10(places - 19)), places)
      } else {
        fixed_text(&round_div(&raw, &pow10(19 - places), mode), places)
      };
      let n = x.write_ascii_dp(&mut buf, places, mode).unwrap();
      prop_assert_eq!(
        std::str::from_utf8(&buf[..n]).unwrap(),
        expected.as_str(),
        "{:?}",
        mode
      );
    }
  }

  #[test]
  fn udec19_fixed_places_match_oracle(raw in uvalue(), places in 0u32..=25) {
    let x = UDec19::from_raw(raw);
    let raw = BigInt::from(raw);
    let mut buf = [0u8; 64];
    for mode in MODES {
      let expected = if places >= 19 {
        fixed_text(&(&raw * pow10(places - 19)), places)
      } else {
        fixed_text(&round_div(&raw, &pow10(19 - places), mode), places)
      };
      let n = x.write_ascii_dp(&mut buf, places, mode).unwrap();
      prop_assert_eq!(
        std::str::from_utf8(&buf[..n]).unwrap(),
        expected.as_str(),
        "{:?}",
        mode
      );
    }
  }

  #[test]
  fn parse_matches_oracle(text in decimal_text()) {
    check_parse(text.as_bytes())?;
  }

  #[test]
  fn parse_arbitrary_bytes_matches_oracle(text in text_bytes()) {
    check_parse(&text)?;
  }

  /// Too-short buffers are rejected at every length, and nothing past the
  /// written text is touched, in either writer.
  #[test]
  fn short_buffers_and_no_writes_past_the_end(raw in value(), places in 0u32..=22) {
    type Writer<'a> = &'a dyn Fn(&mut [u8]) -> Result<usize, BufferTooSmall>;
    let x = Dec19::from_raw(raw);
    let shortest = |out: &mut [u8]| x.write_ascii(out);
    let fixed = |out: &mut [u8]| x.write_ascii_dp(out, places, Round::HalfEven);
    for write in [&shortest as Writer, &fixed] {
      let mut big = [0u8; 64];
      let len = write(&mut big).unwrap();
      let text = big[..len].to_vec();
      for short in 0..len {
        let mut buf = vec![0xAA; short];
        prop_assert_eq!(write(&mut buf), Err(BufferTooSmall));
        prop_assert!(buf.iter().all(|&b| b == 0xAA), "short buffer written to");
      }
      let mut buf = vec![0xAA; len + 16];
      prop_assert_eq!(write(&mut buf), Ok(len));
      prop_assert_eq!(&buf[..len], &text[..]);
      prop_assert!(buf[len..].iter().all(|&b| b == 0xAA), "wrote past the end");
    }
  }
}

#[test]
fn parse_examples() {
  assert_eq!(Dec19::from_ascii(b"113.725"), Ok(dec!(113.725)));
  assert_eq!(Dec19::from_ascii(b"+113.725"), Ok(dec!(113.725)));
  assert_eq!(Dec19::from_ascii(b"-0.5"), Ok(dec!(-0.5)));
  assert_eq!(Dec19::from_ascii(b".5"), Ok(dec!(0.5)));
  assert_eq!(Dec19::from_ascii(b"5."), Ok(dec!(5)));
  assert_eq!(Dec19::from_ascii(b"-0"), Ok(Dec19::ZERO));
  assert_eq!(Dec19::from_ascii(b"007"), Ok(dec!(7)));
  // Zeros past the 19th place are fine; anything else isn't.
  assert_eq!(
    Dec19::from_ascii(b"1.50000000000000000000000"),
    Ok(dec!(1.5))
  );
  assert_eq!(
    Dec19::from_ascii(b"1.00000000000000000001"),
    Err(ParseError::TooPrecise)
  );
  for bad in [
    &b""[..],
    b"-",
    b"+",
    b".",
    b"-.",
    b"1..2",
    b"1.2.3",
    b" 1",
    b"1 ",
    b"1e5",
    b"1_000",
    b"0x10",
    b"--1",
    b"\xFF",
  ] {
    assert_eq!(Dec19::from_ascii(bad), Err(ParseError::Invalid), "{bad:?}");
  }
  assert!("١٢٣".parse::<Dec19>().is_err()); // non-ASCII digits
}

#[test]
fn parse_limits_and_error_order() {
  assert_eq!(Dec19::MAX.to_string().parse::<Dec19>(), Ok(Dec19::MAX));
  assert_eq!(Dec19::MIN.to_string().parse::<Dec19>(), Ok(Dec19::MIN));
  assert_eq!(UDec19::MAX.to_string().parse::<UDec19>(), Ok(UDec19::MAX));
  assert_eq!(
    "17014118346046923173.1687303715884105728".parse::<Dec19>(),
    Err(ParseError::OutOfRange)
  );
  assert_eq!("-1".parse::<UDec19>(), Err(ParseError::OutOfRange));
  // Out of range beats too precise.
  assert_eq!(
    "20000000000000000000.00000000000000000001".parse::<Dec19>(),
    Err(ParseError::OutOfRange)
  );
  // Invalid beats out of range.
  assert_eq!(
    "999999999999999999999999999999999999999999x".parse::<Dec19>(),
    Err(ParseError::Invalid)
  );
  // Rounding up can push a value out of range.
  assert_eq!(
    Dec19::parse_round(
      "17014118346046923173.16873037158841057275",
      Round::Ceiling
    ),
    Err(ParseError::OutOfRange)
  );
  assert_eq!(
    Dec19::parse_round(
      "17014118346046923173.16873037158841057275",
      Round::Floor
    ),
    Ok(Dec19::MAX)
  );
}

#[test]
fn display_formatting() {
  let px = dec!(113.725);
  assert_eq!(format!("{px}"), "113.725");
  assert_eq!(format!("{px:?}"), "Dec19(113.725)");
  assert_eq!(format!("{:?}", udec!(2)), "UDec19(2)");
  assert_eq!(format!("{px:>10}"), "   113.725");
  assert_eq!(format!("{px:<10}|"), "113.725   |");
  assert_eq!(format!("{px:^11}"), "  113.725  ");
  assert_eq!(format!("{px:*>10}"), "***113.725");
  assert_eq!(format!("{px:+}"), "+113.725");
  assert_eq!(format!("{:08}", dec!(-1.5)), "-00001.5");
  assert_eq!(format!("{:+.2}", dec!(0)), "+0.00");
  // Precision pads, or rounds half-even like std does for floats.
  assert_eq!(format!("{px:.4}"), "113.7250");
  assert_eq!(format!("{px:.2}"), "113.72");
  assert_eq!(format!("{:.2}", dec!(0.135)), "0.14");
  assert_eq!(format!("{:.0}", dec!(2.5)), "2");
  assert_eq!(format!("{:.0}", dec!(3.5)), "4");
  assert_eq!(format!("{:.2}", dec!(-0.001)), "0.00");
  assert_eq!(format!("{:.2}", dec!(0.999)), "1.00");
  assert_eq!(format!("{:.21}", dec!(1.5)), "1.500000000000000000000");
  assert_eq!(format!("{:10.1}", dec!(-2.25)), "      -2.2");
  // Absurd precision still works (without width support).
  let wide = format!("{:.120}", dec!(1));
  assert_eq!(wide.len(), 122);
  assert!(wide.starts_with("1.000") && wide.ends_with('0'));
}

#[test]
fn write_examples() {
  let mut buf = [0u8; 64];
  let n = dec!(113.725)
    .write_ascii_dp(&mut buf, 4, Round::HalfEven)
    .unwrap();
  assert_eq!(&buf[..n], b"113.7250");
  let n = dec!(113.725)
    .write_ascii_dp(&mut buf, 0, Round::Floor)
    .unwrap();
  assert_eq!(&buf[..n], b"113");
  let n = dec!(-0.5)
    .write_ascii_dp(&mut buf, 0, Round::HalfEven)
    .unwrap();
  assert_eq!(&buf[..n], b"0");
  let n = dec!(-0.5)
    .write_ascii_dp(&mut buf, 0, Round::Floor)
    .unwrap();
  assert_eq!(&buf[..n], b"-1");
  let n = UDec19::MAX.write_ascii(&mut buf).unwrap();
  assert_eq!(n, UDec19::MAX_ASCII_LEN);
  let n = Dec19::MIN.write_ascii(&mut buf).unwrap();
  assert_eq!(n, Dec19::MAX_ASCII_LEN);
  assert_eq!(
    Dec19::MIN.to_string(),
    "-17014118346046923173.1687303715884105728"
  );
  assert_eq!(D, Dec19::ONE.to_raw());
}
