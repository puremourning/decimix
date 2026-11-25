//! The JSON codecs, checked against the big-integer text oracle.
//!
//! Encoding is checked against `reference_text`, which formats the stored
//! integer with big-integer division and string padding rather than the
//! crate's formatter. Decoding is checked by reading `fixed_text`'s form,
//! all 19 decimal places with trailing zeros, which the encoder never
//! writes, so neither direction is only checked against the other.

#[path = "../../decimix/tests/common/mod.rs"]
mod common;

use capnp::message::Builder;
use capnp_decimix::prelude::*;
use capnp_decimix::{Dec19, UDec19};
use capnp_decimix_tests::basic_capnp;
use num_bigint::BigInt;
use proptest::prelude::*;

const ORDER_ID: u32 = 7;

fn codec() -> capnp_json::Codec<'static> {
  capnp_decimix::json::add_codecs(capnp_json::Codec::new())
}

fn encode(price: Dec19, qty: UDec19) -> String {
  let mut msg = Builder::new_default();
  let mut order = msg.init_root::<basic_capnp::order::Builder<'_>>();
  order.set_id(ORDER_ID);
  order.reborrow().get_price().set_dec19(price);
  order.reborrow().get_qty().set_udec19(qty);
  codec().encode(order.into_reader()).unwrap()
}

/// Reads `json` into an `Order` whose decimals start out non-zero, so that
/// a decimal left out can be seen to be left alone.
fn decode(json: &str) -> capnp::Result<(u32, Dec19, UDec19)> {
  let mut msg = Builder::new_default();
  let mut order = msg.init_root::<basic_capnp::order::Builder<'_>>();
  order.reborrow().get_price().set_dec19(Dec19::from_raw(-1));
  order.reborrow().get_qty().set_udec19(UDec19::from_raw(1));
  codec().decode(json, order.reborrow())?;
  let order = order.into_reader();
  Ok((
    order.get_id(),
    order.get_price().get_dec19(),
    order.get_qty().get_udec19(),
  ))
}

proptest! {
  #![proptest_config(common::config())]

  #[test]
  fn encode_matches_oracle(
    price in common::value(),
    qty in common::uvalue(),
  ) {
    let json = encode(Dec19::from_raw(price), UDec19::from_raw(qty));
    prop_assert_eq!(
      json,
      format!(
        r#"{{"id":{ORDER_ID},"price":"{}","qty":"{}"}}"#,
        common::reference_text(&BigInt::from(price)),
        common::reference_text(&BigInt::from(qty)),
      )
    );
  }

  #[test]
  fn decode_matches_oracle(
    price in common::value(),
    qty in common::uvalue(),
  ) {
    let json = format!(
      r#"{{"id":{ORDER_ID},"price":"{}","qty":"{}"}}"#,
      common::fixed_text(&BigInt::from(price), 19),
      common::fixed_text(&BigInt::from(qty), 19),
    );
    prop_assert_eq!(
      decode(&json).unwrap(),
      (ORDER_ID, Dec19::from_raw(price), UDec19::from_raw(qty))
    );
  }
}

#[test]
fn a_missing_decimal_is_left_alone() {
  assert_eq!(
    decode(r#"{"qty":"2.5"}"#).unwrap(),
    (0, Dec19::from_raw(-1), "2.5".parse().unwrap())
  );
}

#[test]
fn a_decimal_through_another_type_is_still_a_decimal() {
  let mut msg = Builder::new_default();
  let mut quote = msg.init_root::<basic_capnp::quote::Builder<'_>>();
  codec()
    .decode(r#"{"bid":"-0.25"}"#, quote.reborrow())
    .unwrap();
  assert_eq!(
    quote.reborrow_as_reader().get_bid().get_dec19(),
    "-0.25".parse().unwrap()
  );
  assert_eq!(
    codec().encode(quote.into_reader()).unwrap(),
    r#"{"bid":"-0.25"}"#
  );
}

#[test]
fn anything_but_a_valid_decimal_string_is_refused() {
  for json in [
    // A JSON number, even one that is exact.
    r#"{"price":1}"#,
    r#"{"price":1.25}"#,
    // The wire form.
    r#"{"price":{"lo":"0","hi":"0"}}"#,
    r#"{"price":null}"#,
    r#"{"price":"1e3"}"#,
    r#"{"price":" 1"}"#,
    r#"{"price":"0.00000000000000000001"}"#,
    r#"{"price":"17014118346046923174"}"#,
    r#"{"qty":"-1"}"#,
  ] {
    assert!(decode(json).is_err(), "{json} was accepted");
  }
}
