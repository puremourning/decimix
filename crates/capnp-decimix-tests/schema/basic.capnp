@0xfb1311b21053a19a;

using Decimix = import "/decimix.capnp";

struct Simple {
  signed @[0-1] :Decimix.Dec19;
  unsigned @[2-3] :Decimix.UDec19;
}

# The decimals start at word 1, not 0, so tests catch code that assumes a
# group's words are at fixed offsets in the struct. `id` shares no word with
# them, so tests can check that writing a decimal leaves it alone.
struct Order {
  id @0 :UInt32;
  price @[1-2] :Decimix.Dec19;
  qty @[3-4] :Decimix.UDec19;
}

# An "old" schema with no decimal fields: reading it as an `Order` shows what
# a reader sees for a decimal field the writer didn't know about.
struct OrderV0 {
  id @0 :UInt32;
}
