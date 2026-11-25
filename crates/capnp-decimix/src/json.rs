//! Decimals in Cap'n Proto's JSON form, as exact decimal strings.
//!
//! Needs the `json` feature, and works with `capnp-json-newtype`'s `Codec`.
//!
//! Without help, a JSON codec writes a decimal field as what it is on the
//! wire, a group of two words: `{"lo": "1500000000000000000", "hi": "0"}`
//! for 0.15. That is unreadable, and a program in another language would
//! have to rebuild the 128-bit integer to use it. [`add_codecs`] makes the
//! codec write the number itself instead, as a string: `"0.15"`.
//!
//! ```
//! # use capnp_decimix_tests::basic_capnp;
//! use capnp_decimix::prelude::*;
//! use decimix::{dec, udec};
//!
//! # fn main() -> capnp::Result<()> {
//! let codec = capnp_decimix::json::add_codecs(capnp_json::Codec::new());
//!
//! let mut message = capnp::message::Builder::new_default();
//! let mut order = message.init_root::<basic_capnp::order::Builder<'_>>();
//! order.set_id(7);
//! order.reborrow().get_price().set_dec19(dec!(-101.25));
//! order.reborrow().get_qty().set_udec19(udec!(3));
//! let json = codec.encode(order.reborrow_as_reader())?;
//! assert_eq!(json, r#"{"id":7,"price":"-101.25","qty":"3"}"#);
//!
//! codec.decode(r#"{"price":"0.15"}"#, order.reborrow())?;
//! assert_eq!(order.reborrow_as_reader().get_price().get_dec19(), dec!(0.15));
//! # Ok(())
//! # }
//! ```
//!
//! # The format
//!
//! A decimal is written as [`Display`](core::fmt::Display) writes it: the
//! shortest exact form, with no `+`, no trailing zeros after the point, and
//! no point if there is no fraction (`"101.25"`, `"-0.5"`, `"3"`).
//!
//! It is read as [`FromStr`](core::str::FromStr) reads it, so `"+1.50"`,
//! `".5"` and `"5."` are accepted, and an exponent, more than 19 decimal
//! places, a value out of range, and a negative `UDec19` are refused.
//!
//! **A JSON number is refused.** Most JSON libraries read a number into a
//! binary floating-point value, which can't hold most decimals exactly (0.1
//! is not exactly 0.1 as an `f64`), so by the time a number reaches here it
//! may no longer be the value that was meant. A string can only be read
//! exactly. Refusing numbers makes a client that sends them fail at once,
//! rather than lose a fraction of a cent now and then.
//!
//! # Which fields
//!
//! The codecs are matched by the field's declared type, `Decimix.Dec19` or
//! `Decimix.UDec19`, wherever it appears, including through another `type`
//! declared as one of them. As with every `capnp-json` override, a decimal
//! that is the root of the message, or an element of a list, is not
//! matched.

use capnp::dynamic_value;
use capnp_json::{Codec, FieldCodec, JsonValue};

use crate::decimix_capnp::{dec19, u_dec19};

/// `codec` with [`Dec19Json`] and [`UDec19Json`] registered for every
/// `Decimix.Dec19` and `Decimix.UDec19` field.
///
/// Building a `Codec` allocates, so keep the result to convert many
/// messages rather than calling this for each one.
#[must_use]
pub fn add_codecs(codec: Codec<'_>) -> Codec<'_> {
  codec
    .with_newtype_override(dec19::TYPE_ID, Dec19Json)
    .with_newtype_override(u_dec19::TYPE_ID, UDec19Json)
}

/// Writes a `Decimix.Dec19` field as an exact decimal string, and reads one
/// back. See the [module](self) documentation for the format.
#[derive(Clone, Copy, Debug, Default)]
pub struct Dec19Json;

impl FieldCodec for Dec19Json {
  fn encode_value(
    &self,
    source: dynamic_value::Reader<'_>,
  ) -> capnp::Result<JsonValue> {
    let field: dec19::AnyReader<'_> = source.downcast();
    Ok(JsonValue::String(
      crate::dec19::from_capnp(&field).to_string(),
    ))
  }

  fn decode_value(
    &self,
    source: &JsonValue,
    target: dynamic_value::Builder<'_>,
  ) -> capnp::Result<()> {
    let value = parse::<decimix::Dec19>(source)?;
    let mut field: dec19::AnyBuilder<'_> = target.downcast();
    crate::dec19::to_capnp(value, &mut field);
    Ok(())
  }
}

/// Writes a `Decimix.UDec19` field as an exact decimal string, and reads
/// one back. See the [module](self) documentation for the format.
#[derive(Clone, Copy, Debug, Default)]
pub struct UDec19Json;

impl FieldCodec for UDec19Json {
  fn encode_value(
    &self,
    source: dynamic_value::Reader<'_>,
  ) -> capnp::Result<JsonValue> {
    let field: u_dec19::AnyReader<'_> = source.downcast();
    Ok(JsonValue::String(
      crate::udec19::from_capnp(&field).to_string(),
    ))
  }

  fn decode_value(
    &self,
    source: &JsonValue,
    target: dynamic_value::Builder<'_>,
  ) -> capnp::Result<()> {
    let value = parse::<decimix::UDec19>(source)?;
    let mut field: u_dec19::AnyBuilder<'_> = target.downcast();
    crate::udec19::to_capnp(value, &mut field);
    Ok(())
  }
}

/// The decimal a JSON value holds, which must be a string.
fn parse<T>(source: &JsonValue) -> capnp::Result<T>
where
  T: core::str::FromStr,
  T::Err: core::fmt::Display,
{
  let JsonValue::String(text) = source else {
    return Err(capnp::Error::failed(
      "a decimal must be a string, such as \"1.25\", not a JSON number".into(),
    ));
  };
  text.parse().map_err(|error| {
    capnp::Error::failed(format!("invalid decimal {text:?}: {error}"))
  })
}
