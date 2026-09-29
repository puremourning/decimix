//! Maths `Dec19` doesn't do: convert to `BigDecimal` exactly, and back with a
//! named rounding. Run with `--features bigdecimal`.

use bigdecimal::BigDecimal;
use decimix::{Dec19, Round, dec};

/// Standard deviation of some returns: needs a square root.
fn std_dev(xs: &[Dec19]) -> Dec19 {
  let n = BigDecimal::from(xs.len() as u64);
  let big: Vec<BigDecimal> = xs.iter().map(|x| x.to_big()).collect();
  let mean = big.iter().sum::<BigDecimal>() / &n;
  let var = big
    .iter()
    .map(|x| (x - &mean) * (x - &mean))
    .sum::<BigDecimal>()
    / &n;
  let sd = var.sqrt().expect("variance is not negative");
  Dec19::from_big(&sd, Round::HalfEven).expect("in range")
}

fn main() {
  let returns = [dec!(0.012), dec!(-0.004), dec!(0.007), dec!(0.001)];
  let sd = std_dev(&returns);
  println!("std dev = {sd}");

  // Going to BigDecimal is always exact.
  let b: BigDecimal = dec!(113.725).into();
  assert_eq!(b, "113.725".parse::<BigDecimal>().unwrap());

  // Coming back exactly refuses to round...
  let third = BigDecimal::from(1) / BigDecimal::from(3);
  assert!(Dec19::try_from(&third).is_err());
  // ...unless you name the rounding.
  assert_eq!(
    Dec19::from_big(&third, Round::HalfEven),
    Ok(dec!(0.3333333333333333333))
  );
}
