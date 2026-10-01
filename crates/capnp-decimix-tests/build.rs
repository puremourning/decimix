// What the `capnp-decimix` docs tell a user to write, plus `src_prefix`
// because the schema is in `schema/`.
fn main() {
  capnpc::CompilerCommand::new()
    .src_prefix("schema")
    .import_path(capnp_decimix::import_path())
    .crate_provides("capnp_decimix", [capnp_decimix::SCHEMA_ID])
    .file("schema/basic.capnp")
    .run()
    .expect("compiling schema/basic.capnp");
}
