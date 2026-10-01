//! [Cap'n Proto] encoding for [`decimix`]'s [`Dec19`] and [`UDec19`].
//!
//! [Cap'n Proto]: https://capnproto.org
//!
//! This crate ships a schema, `decimix.capnp`, with a type for each of the
//! two decimals, and the Rust code to read and write them. It needs the
//! `newtype` branch of capnproto-rust, which adds schema-level `type`
//! declarations.
//!
//! # Using it in your schema
//!
//! Import `decimix.capnp` and use `Decimix.Dec19` or `Decimix.UDec19` as a
//! field's type. Each value takes two 64-bit words, so the field takes two
//! ordinals, one per word:
//!
//! ```capnp
//! using Decimix = import "/decimix.capnp";
//!
//! struct Order {
//!   id @0 :UInt32;
//!   price @[1-2] :Decimix.Dec19;
//!   qty @[3-4] :Decimix.UDec19;
//! }
//! ```
//!
//! Add `capnp-decimix` to both `[dependencies]` and `[build-dependencies]`.
//! In your `build.rs`, add [`import_path`] (the directory holding
//! `decimix.capnp`) to the import path, and tell the code generator, with
//! [`SCHEMA_ID`], that this crate provides that schema's generated code, so
//! yours refers to it rather than expecting its own copy:
//!
//! ```ignore
//! capnpc::CompilerCommand::new()
//!   .import_path(capnp_decimix::import_path())
//!   .crate_provides("capnp_decimix", [capnp_decimix::SCHEMA_ID])
//!   .file("order.capnp")
//!   .run()
//!   .unwrap();
//! ```
//!
//! If your build system can't use [`import_path`] (see its caveat), copy
//! `decimix.capnp` into your project instead, as is usual for Cap'n Proto
//! schemas, and keep the [`SCHEMA_ID`] line.
//!
//! # Reading and writing
//!
//! With the [`prelude`] in scope, a decimal field's reader has `get_dec19`
//! (or `get_udec19`) and its builder has `set_dec19` (or `set_udec19`):
//!
//! ```
//! # use capnp_decimix_tests::basic_capnp;
//! use capnp_decimix::prelude::*;
//! use decimix::{dec, udec};
//!
//! # fn main() {
//! let mut message = capnp::message::Builder::new_default();
//! let mut order = message.init_root::<basic_capnp::order::Builder<'_>>();
//! order.set_id(7);
//! order.reborrow().get_price().set_dec19(dec!(-101.25));
//! order.reborrow().get_qty().set_udec19(udec!(3));
//!
//! let order = order.into_reader();
//! assert_eq!(order.get_price().get_dec19(), dec!(-101.25));
//! assert_eq!(order.get_qty().get_udec19(), udec!(3));
//! # }
//! ```
//!
//! These work on every field of these types, in any struct, and on the
//! type-erased `as_any()` readers and builders. The same conversions are
//! available as free functions in [`dec19`] and [`udec19`].
//!
//! A field that isn't in the message, e.g. one written by a program using an
//! older schema without it, reads as zero (Cap'n Proto's default). So adding
//! a decimal field to a struct is backward compatible, but a reader can't
//! tell zero from "not sent".
//!
//! # The wire format
//!
//! A value is its stored integer: the value in steps of 10⁻¹⁹, i.e. the
//! value × 10,000,000,000,000,000,000 (see [`Dec19::to_raw`]). That 128-bit
//! integer is split into two `UInt64` fields: `lo` holds the low 64 bits and
//! `hi` the high 64 bits. For `Dec19` the integer is two's complement, so the
//! top bit of `hi` is the sign. For example:
//!
//! | Value               | `hi`                    | `lo`                    |
//! |---------------------|-------------------------|-------------------------|
//! | 1                   | `0`                     | `0x8AC7_2304_89E8_0000` |
//! | −1                  | `0xFFFF_FFFF_FFFF_FFFF` | `0x7538_DCFB_7618_0000` |
//! | −0.000…01 (1 step)  | `0xFFFF_FFFF_FFFF_FFFF` | `0xFFFF_FFFF_FFFF_FFFF` |
//!
//! (1 is stored as 10¹⁹ = `0x8AC7_2304_89E8_0000`, which fits in `lo`; −1 is
//! stored as −10¹⁹, whose low word is 2⁶⁴ − 10¹⁹.)
//!
//! Every pair of words is a valid value, so decoding can't fail, and every
//! value has exactly one encoding.
//!
//! Cap'n Proto stores each `UInt64` little-endian whatever the host, and
//! this crate splits and joins the words arithmetically rather than by
//! reinterpreting memory, so the format is the same on big- and
//! little-endian hosts. Laid out in a struct, the two words are a
//! little-endian 128-bit integer if `lo`'s word comes first, as it does for
//! consecutive ordinals like `@[1-2]`; don't rely on that, read the fields.
//!
//! # Why two words rather than `Data`
//!
//! Cap'n Proto has no 128-bit integer. The alternative is a 16-byte `Data`
//! field holding [`Dec19::to_le_bytes`], but `Data` is stored behind a
//! pointer (an extra indirection and 8 more bytes per value), its length has
//! to be checked at run time, and a type alias for it is still just `Data`
//! in the generated code, so a decimal can't be told apart from any other
//! bytes. A `type` group is stored inline in the struct, and the generated
//! code gives each one its own `Reader` and `Builder` traits, which this
//! crate implements its conversions on.

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub use decimix::{Dec19, UDec19};

/// The directory holding `decimix.capnp`, for a `build.rs` to pass to
/// `capnpc::CompilerCommand::import_path`. Schemas then import it as
/// `"/decimix.capnp"`.
///
/// This is where cargo unpacked this crate's source (the registry, git or
/// path checkout), recorded when the crate was compiled. Build scripts run on
/// the machine that compiled it, so the path exists for them under plain
/// cargo. A build system that compiles in one place and builds in another
/// (e.g. a shared cache of compiled crates, or a sandbox that is removed
/// afterwards) can leave a stale path; copy the schema instead there.
#[must_use]
pub fn import_path() -> &'static std::path::Path {
  std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// The file ID of `decimix.capnp` (its `@0x...` line), for a `build.rs` to
/// pass to `capnpc::CompilerCommand::crate_provides` along with the crate
/// name `"capnp_decimix"`.
pub const SCHEMA_ID: u64 = 0xf534_c073_fd15_b2d4;

/// The code generated from `decimix.capnp`: a `Reader` and `Builder` trait
/// for each type, implemented by every field of that type.
///
/// You rarely need these directly: the [`prelude`] methods are implemented
/// on them.
#[allow(missing_docs)]
pub mod decimix_capnp {
  // What `capnp::generated_code!` expands to, written out so the module can
  // carry documentation.
  #![allow(clippy::all)]
  include!(concat!(env!("OUT_DIR"), "/decimix_capnp.rs"));
}

/// Methods for reading and writing decimal fields: `use
/// capnp_decimix::prelude::*`.
///
/// The traits here are implemented for every reader and builder of a
/// `Decimix.Dec19` or `Decimix.UDec19` field, so you don't name them; they
/// just need to be in scope.
pub mod prelude {
  /// `get_dec19` on a `Decimix.Dec19` field's reader.
  pub trait Dec19Reader<'msg>:
    crate::decimix_capnp::dec19::Reader<'msg>
  where
    Self: Sized,
  {
    /// The field's value. A field missing from the message reads as zero.
    fn get_dec19(&self) -> crate::Dec19 {
      crate::dec19::from_capnp(self)
    }
  }

  impl<'msg, T: crate::decimix_capnp::dec19::Reader<'msg>> Dec19Reader<'msg>
    for T
  {
  }

  /// `set_dec19` on a `Decimix.Dec19` field's builder.
  pub trait Dec19Builder<'msg>:
    crate::decimix_capnp::dec19::Builder<'msg>
  where
    Self: Sized,
  {
    /// Sets the field to `value`, replacing whatever was there.
    fn set_dec19(&mut self, value: crate::Dec19) {
      crate::dec19::to_capnp(value, self)
    }
  }

  impl<'msg, T: crate::decimix_capnp::dec19::Builder<'msg>> Dec19Builder<'msg>
    for T
  {
  }

  /// `get_udec19` on a `Decimix.UDec19` field's reader.
  pub trait UDec19Reader<'msg>:
    crate::decimix_capnp::u_dec19::Reader<'msg>
  where
    Self: Sized,
  {
    /// The field's value. A field missing from the message reads as zero.
    fn get_udec19(&self) -> crate::UDec19 {
      crate::udec19::from_capnp(self)
    }
  }

  impl<'msg, T: crate::decimix_capnp::u_dec19::Reader<'msg>> UDec19Reader<'msg>
    for T
  {
  }

  /// `set_udec19` on a `Decimix.UDec19` field's builder.
  pub trait UDec19Builder<'msg>:
    crate::decimix_capnp::u_dec19::Builder<'msg>
  where
    Self: Sized,
  {
    /// Sets the field to `value`, replacing whatever was there.
    fn set_udec19(&mut self, value: crate::UDec19) {
      crate::udec19::to_capnp(value, self)
    }
  }

  impl<'msg, T: crate::decimix_capnp::u_dec19::Builder<'msg>>
    UDec19Builder<'msg> for T
  {
  }

  pub use crate::{dec19 as dec19_capnp, udec19 as udec19_capnp};
}

/// [`Dec19`] to and from a `Decimix.Dec19` field.
///
/// The [`prelude`] methods call these.
pub mod dec19 {
  /// Reads the value from a field's `lo` and `hi` words.
  pub fn from_capnp<'msg>(
    reader: &impl crate::decimix_capnp::dec19::Reader<'msg>,
  ) -> decimix::Dec19 {
    // Put `hi` in the top 64 bits and `lo` in the bottom 64; the casts to
    // u128 add zeros above each word, so neither spills into the other.
    // Casting the u128 to i128 keeps the bits, so a set top bit in `hi`
    // makes the value negative: two's complement, as it was written.
    decimix::Dec19::from_raw(
      (reader.get_lo() as u128 | (reader.get_hi() as u128) << 64) as i128,
    )
  }

  /// Writes the value into a field's `lo` and `hi` words.
  pub fn to_capnp<'msg>(
    value: decimix::Dec19,
    builder: &mut impl crate::decimix_capnp::dec19::Builder<'msg>,
  ) {
    let raw = value.to_raw();
    // Casting to u64 keeps the bottom 64 bits. Shifting right by 64 first
    // brings the top 64 bits down (filling with copies of the sign bit,
    // which the cast then drops).
    builder.set_lo(raw as u64);
    builder.set_hi((raw >> 64) as u64);
  }
}

/// [`UDec19`] to and from a `Decimix.UDec19` field.
///
/// The [`prelude`] methods call these.
pub mod udec19 {
  /// Reads the value from a field's `lo` and `hi` words.
  pub fn from_capnp<'msg>(
    reader: &impl crate::decimix_capnp::u_dec19::Reader<'msg>,
  ) -> decimix::UDec19 {
    // `hi` in the top 64 bits, `lo` in the bottom 64, as for `Dec19`.
    decimix::UDec19::from_raw(
      reader.get_lo() as u128 | (reader.get_hi() as u128) << 64,
    )
  }

  /// Writes the value into a field's `lo` and `hi` words.
  pub fn to_capnp<'msg>(
    value: decimix::UDec19,
    builder: &mut impl crate::decimix_capnp::u_dec19::Builder<'msg>,
  ) {
    let raw = value.to_raw();
    // The bottom 64 bits, then the top 64 bits.
    builder.set_lo(raw as u64);
    builder.set_hi((raw >> 64) as u64);
  }
}
