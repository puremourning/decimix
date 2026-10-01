//! Test schemas for `capnp-decimix`, compiled the way a user's crate would
//! compile its own: importing `/decimix.capnp` through
//! [`capnp_decimix::import_path`] and pointing the generated code at
//! `capnp_decimix` with [`capnp_decimix::SCHEMA_ID`]. The tests are in
//! `tests/`.

capnp::generated_code!(pub mod basic_capnp);
