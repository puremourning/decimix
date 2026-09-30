//! Shared numeric constants.

/// The stored integer for the value 1: 10¹⁹ = 10,000,000,000,000,000,000.
///
/// Every value is stored as a whole number of 10⁻¹⁹ steps, so 1 is stored as
/// 10¹⁹ and 113.725 as 1,137,250,000,000,000,000,000.
pub(crate) const ONE_RAW: u128 = 10_000_000_000_000_000_000;

/// `POW10[k]` is 10ᵏ, for k from 0 to 38 (10³⁸ is the largest power of ten
/// that fits in a `u128`).
pub(crate) const POW10: [u128; 39] = {
  let mut table = [1u128; 39];
  let mut k = 1;
  while k < 39 {
    table[k] = table[k - 1] * 10;
    k += 1;
  }
  table
};
