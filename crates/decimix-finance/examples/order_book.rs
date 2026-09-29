//! Tick snapping, lots and leaves with domain types.

use decimix::{Round, dec, udec};
use decimix_finance::{Amt, DeltaQty, Price, Qty};

const TICK: Price = Price::new(dec!(0.005));
const LOT: Qty = Qty::new(udec!(100));

fn main() {
  // A model price, snapped passively to the tick grid on each side.
  let fair = Price::new(dec!(113.7268));
  let bid = fair.round_to(TICK, Round::Floor);
  let ask = fair.round_to(TICK, Round::Ceiling);
  assert_eq!(bid, Price::new(dec!(113.725)));
  assert_eq!(ask, Price::new(dec!(113.73)));
  assert_eq!(ask - bid, TICK);

  // Negative prices (spreads, some energy markets) snap the same way.
  let spread_px = Price::new(dec!(-0.0137));
  assert_eq!(
    spread_px.round_to(TICK, Round::Floor),
    Price::new(dec!(-0.015))
  );

  // Price x quantity is an Amt, whichever form the quantity takes,
  // and negative when the price or the quantity is.
  let p = Price::new(dec!(-0.0137));
  let q = Qty::new(udec!(100));
  let cost: Amt = p.mul(q, Round::HalfEven);
  assert_eq!(cost, Amt::new(dec!(-1.37)));
  let short = DeltaQty::new(dec!(-100));
  assert_eq!(bid.mul(short, Round::HalfEven), Amt::new(dec!(-11372.5)));

  // Whole lots in an order, and the odd lot left over.
  let order = Qty::new(udec!(1250));
  assert_eq!(order.div_euclid(LOT), 12);
  assert_eq!(order.rem_euclid(LOT), Qty::new(udec!(50)));

  // Leaves can't go negative; a signed difference is an DeltaQty.
  let filled = Qty::new(udec!(300));
  let leaves = order - filled;
  assert_eq!(leaves, Qty::new(udec!(950)));
  assert_eq!(filled.checked_sub(order), None);
  let delta: DeltaQty = filled.signed_sub(order);
  assert!(delta.is_negative());
  assert_eq!(delta.unsigned_abs(), leaves);

  println!("bid {bid} / ask {ask}, leaves {leaves}, delta {delta}");
}
