//! Allocation-free text in and out, as a FIX engine would use it.

use decimix::{Dec19, Round, UDec19, dec, udec};

/// Appends `tag=value<SOH>` to `out` at `pos`, returning the new position.
fn put_field(
  out: &mut [u8],
  mut pos: usize,
  tag: &[u8],
  value: impl FnOnce(&mut [u8]) -> usize,
) -> usize {
  out[pos..pos + tag.len()].copy_from_slice(tag);
  pos += tag.len();
  out[pos] = b'=';
  pos += 1;
  pos += value(&mut out[pos..]);
  out[pos] = 0x01;
  pos + 1
}

fn main() {
  let px = dec!(113.725);
  let qty = udec!(250);
  let mut msg = [0u8; 128];

  let mut pos = 0;
  // Price (44): shortest exact form.
  pos = put_field(&mut msg, pos, b"44", |buf| px.write_ascii(buf).unwrap());
  // OrderQty (38).
  pos = put_field(&mut msg, pos, b"38", |buf| qty.write_ascii(buf).unwrap());
  // A venue that insists on exactly 4 decimals.
  pos = put_field(&mut msg, pos, b"6", |buf| {
    px.write_ascii_dp(buf, 4, Round::HalfEven).unwrap()
  });
  let encoded = &msg[..pos];
  assert_eq!(encoded, b"44=113.725\x0138=250\x016=113.7250\x01");

  // Decoding: parse straight from the received bytes, no UTF-8 check.
  let mut fields = encoded.split(|&b| b == 0x01).filter(|f| !f.is_empty());
  let price_field = fields.next().unwrap();
  assert_eq!(Dec19::from_ascii(&price_field[3..]), Ok(px)); // after "44="
  let qty_field = fields.next().unwrap();
  assert_eq!(UDec19::from_ascii(&qty_field[3..]), Ok(qty));

  // Or a small stack buffer when that's more convenient.
  let text = px.to_ascii();
  assert_eq!(text.as_bytes(), b"113.725");
  assert_eq!(&*text, "113.725");

  println!("{}", String::from_utf8_lossy(encoded).replace('\x01', "|"));
}
