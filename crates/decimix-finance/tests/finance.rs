//! A light check of the domain types: the products and conversions they add
//! on top of decimix, which has its own thorough tests.

use decimix::{OutOfRange, Round, UDec19, dec, udec};
use decimix_finance::{Amt, DeltaQty, Percentage, Price, Qty};

const HE: Round = Round::HalfEven;

#[test]
fn products() {
  let px = Price::new(dec!(113.725));
  assert_eq!(px.mul(Qty::new(udec!(250)), HE), Amt::new(dec!(28431.25)));
  assert_eq!(
    px.mul(DeltaQty::new(dec!(-250)), HE),
    Amt::new(dec!(-28431.25))
  );
  // Rounds like the base types do: 0.125 * 0.1 = 0.0125 exactly, but
  // 1/3-ish values round with the given mode.
  let third = Price::new(dec!(0.3333333333333333333));
  assert_eq!(
    third.mul(Qty::new(udec!(0.5)), HE),
    Amt::new(dec!(0.1666666666666666666))
  );
  assert_eq!(
    third.mul(Qty::new(udec!(0.5)), Round::Ceiling),
    Amt::new(dec!(0.1666666666666666667))
  );
  let fee = Amt::new(dec!(28431.25)).mul(Percentage::new(dec!(0.0002)), HE);
  assert_eq!(fee, Amt::new(dec!(5.68625)));
  assert_eq!(
    Price::new(decimix::Dec19::MAX).checked_mul(Qty::new(udec!(2)), HE),
    None
  );
}

#[test]
fn sums_and_vwap() {
  let fills = [
    (Price::new(dec!(113.725)), Qty::new(udec!(100))),
    (Price::new(dec!(113.73)), Qty::new(udec!(250))),
  ];
  assert_eq!(Amt::sum_products(fills, HE), Ok(Amt::new(dec!(39805))));
  assert_eq!(
    Price::vwap(fills, HE),
    Some(Price::new(dec!(113.7285714285714285714)))
  );
  // Net amount over signed quantities.
  let trades = [
    (Price::new(dec!(10)), DeltaQty::new(dec!(3))),
    (Price::new(dec!(11)), DeltaQty::new(dec!(-3))),
  ];
  assert_eq!(Amt::sum_products(trades, HE), Ok(Amt::new(dec!(-3))));
  // No fills, or no quantity: no VWAP.
  assert_eq!(Price::vwap([], HE), None);
  assert_eq!(Price::vwap([(Price::new(dec!(1)), Qty::ZERO)], HE), None);
  // The running totals overflowing: no VWAP or sum, rather than a panic.
  let max = (Price::new(decimix::Dec19::MAX), Qty::new(UDec19::MAX));
  assert_eq!(Price::vwap([max, max, max], HE), None);
  assert_eq!(Amt::sum_products([max, max, max], HE), Err(OutOfRange));
  // Total quantity overflowing: no VWAP rather than a panic.
  let huge = Qty::new(UDec19::MAX);
  assert_eq!(
    Price::vwap(
      [(Price::new(dec!(0)), huge), (Price::new(dec!(0)), huge)],
      HE
    ),
    None
  );
}

#[test]
fn quantity_conversions() {
  let (a, b) = (Qty::new(udec!(250)), Qty::new(udec!(1000)));
  assert_eq!(a.signed_sub(b), DeltaQty::new(dec!(-750)));
  assert_eq!(a.to_delta_qty(), DeltaQty::new(dec!(250)));
  assert_eq!(
    DeltaQty::new(dec!(-750)).unsigned_abs(),
    Qty::new(udec!(750))
  );
  assert_eq!(
    DeltaQty::new(decimix::Dec19::MIN)
      .unsigned_abs()
      .get()
      .to_raw(),
    1 << 127
  );
  assert_eq!(Qty::try_from(DeltaQty::new(dec!(-1))), Err(OutOfRange));
  assert_eq!(DeltaQty::try_from(Qty::new(UDec19::MAX)), Err(OutOfRange));
  assert!(
    std::panic::catch_unwind(|| Qty::new(UDec19::MAX).to_delta_qty()).is_err()
  );
}

#[test]
fn newtype_basics() {
  const TICK: Price = Price::new(dec!(0.005));
  let px = Price::new(dec!(-1.2345));
  assert_eq!(px.round_to(TICK, Round::Floor), Price::new(dec!(-1.235)));
  assert_eq!(-px, Price::new(dec!(1.2345)));
  assert_eq!(px.to_string(), "-1.2345");
  assert_eq!(format!("{px:?}"), "Price(-1.2345)");
  assert_eq!("-1.2345".parse::<Price>(), Ok(px));
  assert_eq!(Qty::new(udec!(1250)).div_euclid(Qty::new(udec!(100))), 12);
  assert_eq!(Qty::from(3) * 2, Qty::new(udec!(6)));
}
