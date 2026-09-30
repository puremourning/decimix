//! Amounts, fees and VWAP over a set of fills.

use decimix::{Round, dec, udec};
use decimix_finance::{Amt, DeltaQty, Percentage, Price, Qty};

fn main() {
  let fills = [
    (Price::new(dec!(113.725)), Qty::new(udec!(100))),
    (Price::new(dec!(113.73)), Qty::new(udec!(250))),
    (Price::new(dec!(113.74)), Qty::new(udec!(50))),
  ];

  // One fill: price x quantity is the only product, and it names its rounding.
  let (px, qty) = fills[0];
  let n: Amt = px.mul(qty, Round::HalfEven);
  assert_eq!(n, Amt::new(dec!(11372.5)));

  // All fills: every product kept exactly, one rounding at the end.
  let total = Amt::sum_products(fills, Round::HalfEven).unwrap();
  assert_eq!(total, Amt::new(dec!(45492.0)));

  let fee = total.mul(Percentage::new(dec!(0.0002)), Round::HalfEven);
  let fee = fee.round_dp(2, Round::HalfEven);
  assert_eq!(fee, Amt::new(dec!(9.10)));

  let vwap = Price::vwap(fills, Round::HalfEven).unwrap();
  assert_eq!(vwap, Price::new(dec!(113.73)));

  // Net amount over signed quantities: buys positive, sells negative.
  let trades = [
    (Price::new(dec!(113.725)), DeltaQty::new(dec!(100))),
    (Price::new(dec!(113.75)), DeltaQty::new(dec!(-100))),
  ];
  let net = Amt::sum_products(trades, Round::HalfEven).unwrap();
  assert_eq!(net, Amt::new(dec!(-2.5))); // bought low, sold high

  let filled: Qty = fills.iter().map(|&(_, q)| q).sum();
  println!("filled {filled} @ {vwap}, amount {total}, fee {fee}");
}
