//! Arbitrary bytes into both parsers. Whatever is accepted must round-trip
//! through text, and rounding must agree with exact parsing when that
//! succeeds.
#![no_main]

use decimix::{Dec19, UDec19};
use decimix_fuzz::MODES;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
  if let Ok(x) = Dec19::from_ascii(data) {
    assert_eq!(Dec19::from_ascii(x.to_string().as_bytes()), Ok(x));
    for mode in MODES {
      assert_eq!(Dec19::from_ascii_round(data, mode), Ok(x));
    }
  }
  if let Ok(x) = UDec19::from_ascii(data) {
    assert_eq!(UDec19::from_ascii(x.to_string().as_bytes()), Ok(x));
  }
  for mode in MODES {
    if let Ok(x) = Dec19::from_ascii_round(data, mode) {
      assert_eq!(Dec19::from_ascii(x.to_string().as_bytes()), Ok(x));
    }
    let _ = UDec19::from_ascii_round(data, mode);
  }
  if let Ok(s) = core::str::from_utf8(data) {
    assert_eq!(s.parse::<Dec19>(), Dec19::from_ascii(data));
  }
});
