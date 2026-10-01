fn main() {
  capnpc::CompilerCommand::new()
    .file("decimix.capnp")
    .run()
    .expect("compiling decimix.capnp");
}
