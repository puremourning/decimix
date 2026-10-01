//! The wire format, checked against a big-integer oracle.
//!
//! Each value is its stored integer (the value × 10¹⁹) in two `UInt64`
//! words: `lo` is the low 64 bits, `hi` the high 64 bits, and for `Dec19`
//! the 128 bits are two's complement. The crate splits and joins the words
//! with shifts and casts; the oracle does it with `BigInt` division and
//! remainder instead, so the two can't share a mistake:
//!
//! - lo = stored mod 2⁶⁴, hi = ⌊stored / 2⁶⁴⌋ mod 2⁶⁴ (both floored, so a
//!   negative stored value wraps the way two's complement does);
//! - stored = lo + hi × 2⁶⁴, minus 2¹²⁸ if that is 2¹²⁷ or more and the
//!   type is signed (the top bit is the sign).
//!
//! Encoding is checked by reading the words straight out of the serialized
//! message bytes, and decoding by building those bytes by hand, so neither
//! direction is only checked against the other.

#[path = "../../decimix/tests/common/mod.rs"]
mod common;

use capnp::message::{Builder, ReaderOptions};
use capnp::serialize;
use capnp_decimix::prelude::*;
use capnp_decimix::{Dec19, UDec19};
use capnp_decimix_tests::basic_capnp;
use num_bigint::BigInt;
use num_integer::Integer;
use proptest::prelude::*;

/// 2⁶⁴ = 18,446,744,073,709,551,616: one word's worth.
fn two_pow_64() -> BigInt {
  BigInt::from(1) << 64
}

/// The oracle's split of a stored integer into (lo, hi) words.
fn split(stored: &BigInt) -> (u64, u64) {
  let word = two_pow_64();
  // Floored division and remainder: for -1, lo = 2⁶⁴ - 1 and the quotient
  // is -1, whose remainder is again 2⁶⁴ - 1. That is two's complement.
  let (upper, lo) = stored.div_mod_floor(&word);
  let hi = upper.mod_floor(&word);
  (lo.try_into().unwrap(), hi.try_into().unwrap())
}

/// The oracle's join of (lo, hi) words into a stored integer.
fn join(lo: u64, hi: u64, signed: bool) -> BigInt {
  let unsigned = BigInt::from(lo) + BigInt::from(hi) * two_pow_64();
  if signed && unsigned >= BigInt::from(1) << 127 {
    unsigned - (BigInt::from(1) << 128)
  } else {
    unsigned
  }
}

// A serialized `Order` is one segment: 8 bytes of segment table, an 8-byte
// root pointer, then the struct's five data words. Word 0 holds `id`.
const ID: usize = 16;
const PRICE_LO: usize = 24;
const PRICE_HI: usize = 32;
const QTY_LO: usize = 40;
const QTY_HI: usize = 48;
const ORDER_LEN: usize = 56;

fn word_at(bytes: &[u8], at: usize) -> u64 {
  u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap())
}

const ORDER_ID: u32 = 0xA5A5_A5A5;

/// Serializes an `Order` with the given decimals.
fn encode(price: Dec19, qty: UDec19) -> Vec<u8> {
  let mut msg = Builder::new_default();
  let mut order = msg.init_root::<basic_capnp::order::Builder<'_>>();
  order.set_id(ORDER_ID);
  order.reborrow().get_price().set_dec19(price);
  order.reborrow().get_qty().set_udec19(qty);
  let bytes = serialize::write_message_to_words(&msg);
  assert_eq!(bytes.len(), ORDER_LEN);
  bytes
}

/// Builds a serialized `Order` by hand with the given words.
fn order_bytes(price: (u64, u64), qty: (u64, u64)) -> Vec<u8> {
  // Take the segment table and root pointer from a real message: they
  // depend only on the struct's size, not on its contents.
  let mut bytes = encode(Dec19::ZERO, UDec19::ZERO);
  for (at, word) in [
    (PRICE_LO, price.0),
    (PRICE_HI, price.1),
    (QTY_LO, qty.0),
    (QTY_HI, qty.1),
  ] {
    bytes[at..at + 8].copy_from_slice(&word.to_le_bytes());
  }
  bytes
}

fn read(bytes: &[u8]) -> capnp::message::Reader<serialize::OwnedSegments> {
  serialize::read_message(bytes, ReaderOptions::new()).unwrap()
}

proptest! {
  #![proptest_config(common::config())]

  #[test]
  fn encode_matches_oracle(
    price in common::value(),
    qty in common::uvalue(),
  ) {
    let bytes = encode(Dec19::from_raw(price), UDec19::from_raw(qty));
    let price_words = (word_at(&bytes, PRICE_LO), word_at(&bytes, PRICE_HI));
    let qty_words = (word_at(&bytes, QTY_LO), word_at(&bytes, QTY_HI));
    prop_assert_eq!(price_words, split(&BigInt::from(price)));
    prop_assert_eq!(qty_words, split(&BigInt::from(qty)));
    prop_assert_eq!(join(price_words.0, price_words.1, true), price.into());
    prop_assert_eq!(join(qty_words.0, qty_words.1, false), qty.into());
    // The neighbouring field is untouched.
    prop_assert_eq!(word_at(&bytes, ID), ORDER_ID as u64);
  }

  #[test]
  fn decode_matches_oracle(
    price in common::value(),
    qty in common::uvalue(),
  ) {
    let msg = read(&order_bytes(
      split(&BigInt::from(price)),
      split(&BigInt::from(qty)),
    ));
    let order = msg.get_root::<basic_capnp::order::Reader<'_>>().unwrap();
    let (price, qty) = (Dec19::from_raw(price), UDec19::from_raw(qty));

    prop_assert_eq!(order.get_price().get_dec19(), price);
    prop_assert_eq!(order.get_qty().get_udec19(), qty);
    // The same through the free functions and the type-erased readers.
    prop_assert_eq!(dec19_capnp::from_capnp(&order.get_price()), price);
    prop_assert_eq!(udec19_capnp::from_capnp(&order.get_qty()), qty);
    prop_assert_eq!(order.get_price().as_any().get_dec19(), price);
    prop_assert_eq!(order.get_qty().as_any().get_udec19(), qty);
  }

  /// Any two words decode to the value whose stored integer they spell:
  /// every bit pattern is a valid value, so decoding never fails.
  #[test]
  fn any_words_decode(lo in any::<u64>(), hi in any::<u64>()) {
    let msg = read(&order_bytes((lo, hi), (lo, hi)));
    let order = msg.get_root::<basic_capnp::order::Reader<'_>>().unwrap();
    let price: BigInt = order.get_price().get_dec19().to_raw().into();
    let qty: BigInt = order.get_qty().get_udec19().to_raw().into();
    prop_assert_eq!(price, join(lo, hi, true));
    prop_assert_eq!(qty, join(lo, hi, false));
  }

  /// Writing through the free functions and the type-erased builders gives
  /// the same bytes as the prelude methods.
  #[test]
  fn write_paths_agree(
    price in common::value(),
    qty in common::uvalue(),
  ) {
    let (price, qty) = (Dec19::from_raw(price), UDec19::from_raw(qty));
    let expected = encode(price, qty);

    let mut msg = Builder::new_default();
    let mut order = msg.init_root::<basic_capnp::order::Builder<'_>>();
    order.set_id(ORDER_ID);
    dec19_capnp::to_capnp(price, &mut order.reborrow().get_price());
    udec19_capnp::to_capnp(qty, &mut order.reborrow().get_qty());
    prop_assert_eq!(&serialize::write_message_to_words(&msg), &expected);

    let mut msg = Builder::new_default();
    let mut order = msg.init_root::<basic_capnp::order::Builder<'_>>();
    order.set_id(ORDER_ID);
    order.reborrow().get_price().as_any().set_dec19(price);
    order.reborrow().get_qty().as_any().set_udec19(qty);
    prop_assert_eq!(&serialize::write_message_to_words(&msg), &expected);
  }
}

/// Worked examples, so the format can be read off the test.
#[test]
fn known_words() {
  // 1 is stored as 10¹⁹ = 0x8AC7_2304_89E8_0000, which fits in `lo`.
  let bytes = encode(Dec19::ONE, UDec19::ONE);
  assert_eq!(word_at(&bytes, PRICE_LO), 0x8AC7_2304_89E8_0000);
  assert_eq!(word_at(&bytes, PRICE_HI), 0);

  // -1 is stored as -10¹⁹: in two's complement, `lo` is
  // 2⁶⁴ - 10¹⁹ = 0x7538_DCFB_7618_0000 and `hi` is all ones.
  let bytes = encode(-Dec19::ONE, UDec19::ONE);
  assert_eq!(word_at(&bytes, PRICE_LO), 0x7538_DCFB_7618_0000);
  assert_eq!(word_at(&bytes, PRICE_HI), u64::MAX);

  // The smallest step below zero is all ones in both words.
  let bytes = encode(-Dec19::SMALLEST_STEP, UDec19::MAX);
  assert_eq!(word_at(&bytes, PRICE_LO), u64::MAX);
  assert_eq!(word_at(&bytes, PRICE_HI), u64::MAX);
  assert_eq!(word_at(&bytes, QTY_LO), u64::MAX);
  assert_eq!(word_at(&bytes, QTY_HI), u64::MAX);

  // The limits differ only in the top bit of `hi`.
  let bytes = encode(Dec19::MIN, UDec19::ZERO);
  assert_eq!(word_at(&bytes, PRICE_LO), 0);
  assert_eq!(word_at(&bytes, PRICE_HI), 1 << 63);
  let bytes = encode(Dec19::MAX, UDec19::ZERO);
  assert_eq!(word_at(&bytes, PRICE_LO), u64::MAX);
  assert_eq!(word_at(&bytes, PRICE_HI), u64::MAX >> 1);
}

/// Setting a value replaces both words: nothing of the old value survives.
#[test]
fn set_overwrites() {
  let mut msg = Builder::new_default();
  let mut order = msg.init_root::<basic_capnp::order::Builder<'_>>();
  order.reborrow().get_price().set_dec19(Dec19::MIN);
  order.reborrow().get_qty().set_udec19(UDec19::MAX);
  order.reborrow().get_price().set_dec19(Dec19::SMALLEST_STEP);
  order.reborrow().get_qty().set_udec19(UDec19::ZERO);

  let order = order.into_reader();
  assert_eq!(order.get_price().get_dec19(), Dec19::SMALLEST_STEP);
  assert_eq!(order.get_qty().get_udec19(), UDec19::ZERO);
}

/// A builder can be read back before the message is finished.
#[test]
fn read_from_builder() {
  let mut msg = Builder::new_default();
  let mut order = msg.init_root::<basic_capnp::order::Builder<'_>>();
  order
    .reborrow()
    .get_price()
    .set_dec19(decimix::dec!(-101.25));
  order.reborrow().get_qty().set_udec19(decimix::udec!(3));

  let reader = order.reborrow_as_reader();
  assert_eq!(reader.get_price().get_dec19(), decimix::dec!(-101.25));
  assert_eq!(reader.get_qty().get_udec19(), decimix::udec!(3));
}

/// A message from a writer whose schema had no decimal fields reads them as
/// zero, the Cap'n Proto default. Adding a decimal field is
/// backward compatible, but zero is indistinguishable from "not sent".
#[test]
fn missing_fields_read_as_zero() {
  let mut msg = Builder::new_default();
  msg
    .init_root::<basic_capnp::order_v0::Builder<'_>>()
    .set_id(ORDER_ID);
  let bytes = serialize::write_message_to_words(&msg);

  let msg = read(&bytes);
  let order = msg.get_root::<basic_capnp::order::Reader<'_>>().unwrap();
  assert_eq!(order.get_id(), ORDER_ID);
  assert_eq!(order.get_price().get_dec19(), Dec19::ZERO);
  assert_eq!(order.get_qty().get_udec19(), UDec19::ZERO);
}
