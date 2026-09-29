//! A tour of the core API.

use decimix::{Dec19, Round, UDec19, dec, udec};

// Literals are checked at compile time and never pass through f64.
const TICK: Dec19 = dec!(0.005);

fn main() {
  let bid = dec!(113.725);
  let ask = dec!("113.74");

  // Exact operations are operators: + - unary-minus, comparisons, * integer.
  let spread = ask - bid;
  assert_eq!(spread, dec!(0.015));
  assert_eq!(spread * 2, dec!(0.03));
  assert!(bid < ask);
  let total: Dec19 = [bid, ask].iter().sum();
  assert_eq!(total, dec!(227.465));

  // Overflow panics; checked_* and saturating_* don't.
  assert_eq!(Dec19::MAX.checked_add(Dec19::SMALLEST_STEP), None);
  assert_eq!(Dec19::MAX.saturating_add(Dec19::ONE), Dec19::MAX);

  // Anything that can round is a method with a mandatory Round.
  let mid = total.div_int(2, Round::HalfEven);
  assert_eq!(mid, dec!(113.7325));
  assert_eq!(mid.round_to(TICK, Round::Floor), dec!(113.73));
  assert_eq!(mid.round_to(TICK, Round::Ceiling), dec!(113.735));
  assert_eq!(mid.round_dp(2, Round::HalfEven), dec!(113.73));
  assert_eq!(mid.to_int(Round::TowardZero), 113);

  // Negative prices snap correctly: Floor always moves toward -infinity.
  assert_eq!(dec!(-1.2345).round_to(TICK, Round::Floor), dec!(-1.235));

  // Lots: exact division with a remainder, no rounding.
  let lot = dec!(100);
  let qty = dec!(1250);
  assert_eq!(qty.div_floor(lot), 12);
  assert_eq!(qty.rem_euclid(lot), dec!(50));

  // Decimal x decimal rounds, so it's a method too.
  let notional = bid.mul(dec!(1234.5678901), Round::HalfEven);
  println!("notional = {notional}");

  // Text in and out is exact and locale-free.
  assert_eq!(mid.to_string(), "113.7325");
  assert_eq!(format!("{mid:.2}"), "113.73");
  assert_eq!(format!("{mid:?}"), "Dec19(113.7325)");
  assert_eq!("113.7325".parse::<Dec19>(), Ok(mid));
  assert!("0.12345678901234567891".parse::<Dec19>().is_err()); // 20 places
  assert_eq!(
    Dec19::parse_round("0.12345678901234567891", Round::HalfEven),
    Ok(dec!(0.1234567890123456789))
  );

  // Storage and wire: the stored integer, and implied-decimal integers.
  assert_eq!(bid.to_raw(), 1_137_250_000_000_000_000_000);
  assert_eq!(Dec19::from_raw(bid.to_raw()), bid);
  assert_eq!(Dec19::from_scaled(1_137_250, 4), Some(bid));
  assert_eq!(bid.to_scaled(2, Round::HalfEven), Ok(11_372));

  // Unsigned values for things that can't be negative.
  let ordered = udec!(1000);
  let filled = udec!(250.5);
  let leaves: UDec19 = ordered - filled;
  assert_eq!(leaves, udec!(749.5));
  assert_eq!(filled.checked_sub(ordered), None);
  assert_eq!(filled.signed_sub(ordered), dec!(-749.5));

  println!("bid {bid}, ask {ask}, mid {mid}, spread {spread}");
}
