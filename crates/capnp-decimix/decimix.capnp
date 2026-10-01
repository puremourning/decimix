@0xf534c073fd15b2d4;

type Dec19 = group {
  # A signed decimal number with 19 decimal places and 39 significant digits.
  # Encoded as a little-endian 128 bit integer with bytes 0-7 in `lo` and bytes 8-15 in `hi`.
  lo @0 :UInt64;
  hi @1 :UInt64;
}

type UDec19 = group {
  # An unsigned decimal number with 19 decimal places and 39 significant digits.
  # Encoded as a little-endian 128 bit integer with bytes 0-7 in `lo` and bytes 8-15 in `hi`.
  lo @0 :UInt64;
  hi @1 :UInt64;
}
