//! See Cargo.toml: a fixture for checking the recommended float lint.

use decimix::Dec19;

/// An edge of the system that opts in explicitly: must pass clippy.
#[allow(clippy::disallowed_methods)]
pub fn allowed(x: Dec19) -> f64 {
  x.to_f64_lossy()
}

/// Each banned method, without opting in: clippy must reject every one.
#[cfg(feature = "violate")]
pub fn violations(
  x: Dec19,
  u: decimix::UDec19,
  f: f64,
) -> Result<f64, decimix::FromF64Error> {
  use decimix::{Round, UDec19};
  let a = x.to_f64_lossy();
  let b = u.to_f64_lossy();
  let c = Dec19::from_f64_lossy(f, Dec19::SMALLEST_STEP, Round::HalfEven)?;
  let d = UDec19::from_f64_lossy(f, UDec19::SMALLEST_STEP, Round::HalfEven)?;
  Ok(a + b + allowed(c) + allowed(Dec19::try_from(d).unwrap_or_default()))
}
