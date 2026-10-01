use capnp_decimix_tests::basic_capnp;

#[test]
fn test_simple_encode_decode_one() {
  let mut msg = capnp::message::Builder::new_default();
  let mut builder = msg.init_root::<basic_capnp::simple::Builder<'_>>();

  capnp_decimix::dec19::to_capnp(
    decimix::Dec19::ONE,
    &mut builder.reborrow().get_signed(),
  );
  capnp_decimix::udec19::to_capnp(
    decimix::UDec19::ONE,
    &mut builder.reborrow().get_unsigned(),
  );

  let reader = msg
    .get_root_as_reader::<basic_capnp::simple::Reader<'_>>()
    .unwrap();

  assert_eq!(
    capnp_decimix::dec19::from_capnp(&reader.get_signed()),
    decimix::Dec19::ONE
  );
  assert_eq!(
    capnp_decimix::udec19::from_capnp(&reader.get_unsigned()),
    decimix::UDec19::ONE
  );
}

#[test]
fn test_simple_encode_decode_decimal() {
  let mut msg = capnp::message::Builder::new_default();
  let mut builder = msg.init_root::<basic_capnp::simple::Builder<'_>>();

  capnp_decimix::dec19::to_capnp(
    decimix::dec!(-1.772020),
    &mut builder.reborrow().get_signed(),
  );
  capnp_decimix::udec19::to_capnp(
    decimix::udec!(1.772020),
    &mut builder.reborrow().get_unsigned(),
  );

  let reader = msg
    .get_root_as_reader::<basic_capnp::simple::Reader<'_>>()
    .unwrap();

  assert_eq!(
    capnp_decimix::dec19::from_capnp(&reader.get_signed()),
    decimix::dec!(-1.772020)
  );
  assert_eq!(
    capnp_decimix::udec19::from_capnp(&reader.get_unsigned()),
    decimix::udec!(1.772020)
  );
}

#[test]
fn test_simple_encode_decode_one_prelude() {
  use capnp_decimix::prelude::*;
  let mut msg = capnp::message::Builder::new_default();
  let mut builder = msg.init_root::<basic_capnp::simple::Builder<'_>>();

  builder
    .reborrow()
    .get_signed()
    .set_dec19(decimix::Dec19::ONE);
  builder
    .reborrow()
    .get_unsigned()
    .set_udec19(decimix::UDec19::ONE);

  let reader = msg
    .get_root_as_reader::<basic_capnp::simple::Reader<'_>>()
    .unwrap();

  assert_eq!(
    capnp_decimix::dec19::from_capnp(&reader.get_signed()),
    decimix::Dec19::ONE
  );
  assert_eq!(
    capnp_decimix::udec19::from_capnp(&reader.get_unsigned()),
    decimix::UDec19::ONE
  );
}

#[test]
fn test_simple_encode_decode_decimal_prelude() {
  use capnp_decimix::prelude::*;
  let mut msg = capnp::message::Builder::new_default();
  let mut builder = msg.init_root::<basic_capnp::simple::Builder<'_>>();

  builder
    .reborrow()
    .get_signed()
    .set_dec19(decimix::dec!(-1.772020));
  builder
    .reborrow()
    .get_unsigned()
    .set_udec19(decimix::udec!(1.772020));

  let reader = msg
    .get_root_as_reader::<basic_capnp::simple::Reader<'_>>()
    .unwrap();

  assert_eq!(
    capnp_decimix::dec19::from_capnp(&reader.get_signed()),
    decimix::dec!(-1.772020)
  );
  assert_eq!(
    capnp_decimix::udec19::from_capnp(&reader.get_unsigned()),
    decimix::udec!(1.772020)
  );
}
