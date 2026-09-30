//! Crossing into and out of `f64`: allowed only at the edges, and loud.
//!
//! In a crate whose `clippy.toml` bans these methods (as the decimix docs
//! recommend for core crates), each use needs the `#[allow]` shown here, which
//! makes every float boundary easy to find in review.

use decimix::{Dec19, FromF64Error, Round, dec};

const TICK: Dec19 = dec!(0.005);

/// A market-data feed that sends doubles.
fn on_feed_price(raw: f64) -> Result<Dec19, FromF64Error> {
  // `step` states the precision we trust: here, the instrument's tick.
  #[allow(clippy::disallowed_methods)]
  Dec19::from_f64_lossy(raw, TICK, Round::HalfEven)
}

/// A downstream risk API that wants doubles.
fn to_risk_api(px: Dec19) -> f64 {
  #[allow(clippy::disallowed_methods)]
  px.to_f64_lossy()
}

fn main() {
  // Output: the nearest double, bit for bit what parsing "113.725" gives.
  let px = dec!(113.725);
  assert_eq!(to_risk_api(px), 113.725_f64);

  // Input: 0.1 + 0.2 computed upstream arrives as 0.30000000000000004.
  let noisy = 0.1_f64 + 0.2_f64;
  let clean = Dec19::from_f64_lossy(noisy, dec!(0.000001), Round::HalfEven);
  assert_eq!(clean, Ok(dec!(0.3)));
  // With SMALLEST_STEP you keep every digit of the double's shortest form.
  let kept =
    Dec19::from_f64_lossy(noisy, Dec19::SMALLEST_STEP, Round::HalfEven);
  assert_eq!(kept, Ok(dec!(0.30000000000000004)));

  // Snapping a feed price to the tick grid.
  assert_eq!(on_feed_price(113.7249999999), Ok(dec!(113.725)));

  // Bad doubles are errors, not garbage.
  assert_eq!(on_feed_price(f64::NAN), Err(FromF64Error::NotFinite));
  assert_eq!(on_feed_price(1e300), Err(FromF64Error::OutOfRange));

  // Round trip: any value with up to 15 significant digits survives.
  let x = dec!(98765.4321);
  let back = Dec19::from_f64_lossy(
    x.to_f64_lossy(),
    Dec19::SMALLEST_STEP,
    Round::HalfEven,
  );
  assert_eq!(back, Ok(x));
}
