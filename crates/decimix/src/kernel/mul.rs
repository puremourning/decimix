//! Fixed-scale-19 decimal multiply: i128 x i128 -> 256-bit -> / 10^19 -> i128.
//! Verified against Python big-int reference on 200k random + edge cases,
//! half-even and floor rounding, release and debug builds (rustc 1.91).
//! The hardware-`div` variant (`mul19_hw`) uses inline asm, so it only exists
//! on x86_64, and not under miri.
//!
//! Imported from the design discussion (see `docs/design-brief.md`); apart from
//! the `cfg` gating and formatting, the code is unchanged from the verified
//! version.
#[cfg(all(target_arch = "x86_64", not(miri)))]
use std::arch::asm;
pub const D: u64 = 10_000_000_000_000_000_000; // 10^19, top bit set => already "normalized"
// Möller–Granlund reciprocal: floor((2^128 - 1) / d) - 2^64
const V: u64 = (u128::MAX / D as u128 - (1u128 << 64)) as u64;

#[derive(Clone, Copy)]
pub enum Round {
  HalfEven,
  Floor,
}

/// (u1:u0) / D  with u1 < D. Two multiplies, no hardware divide.
#[inline(always)]
fn div2by1(u1: u64, u0: u64) -> (u64, u64) {
  let q =
    (V as u128 * u1 as u128).wrapping_add(((u1 as u128) << 64) | u0 as u128);
  let (mut q1, q0) = (((q >> 64) as u64).wrapping_add(1), q as u64);
  let mut r = u0.wrapping_sub(q1.wrapping_mul(D));
  if r > q0 {
    q1 = q1.wrapping_sub(1);
    r = r.wrapping_add(D);
  }
  if r >= D {
    q1 += 1;
    r -= D;
  }
  (q1, r)
}

/// Same step using the hardware `div` instruction.
#[cfg(all(target_arch = "x86_64", not(miri)))]
#[inline(always)]
fn div2by1_hw(u1: u64, u0: u64) -> (u64, u64) {
  let (q, r);
  unsafe {
    asm!("div {d}", d = in(reg) D, inout("rax") u0 => q, inout("rdx") u1 => r,
                  options(pure, nomem, nostack));
  }
  (q, r)
}

/// 128x128 -> 256 unsigned, limbs little-endian.
#[inline(always)]
fn mul_128x128(x: u128, y: u128) -> [u64; 4] {
  let (x0, x1, y0, y1) = (
    x as u64 as u128,
    (x >> 64) as u64 as u128,
    y as u64 as u128,
    (y >> 64) as u64 as u128,
  );
  let (p00, p01, p10, p11) = (x0 * y0, x0 * y1, x1 * y0, x1 * y1);
  let mid = (p00 >> 64) + (p01 as u64 as u128) + (p10 as u64 as u128);
  let hi = (mid >> 64) + (p01 >> 64) + (p10 >> 64) + p11; // can't overflow: product < 2^256
  [p00 as u64, mid as u64, hi as u64, (hi >> 64) as u64]
}

#[inline(always)]
fn mul19_impl(
  a: i128,
  b: i128,
  mode: Round,
  div: fn(u64, u64) -> (u64, u64),
) -> Option<i128> {
  let neg = (a < 0) ^ (b < 0);
  let [w0, w1, w2, w3] = mul_128x128(a.unsigned_abs(), b.unsigned_abs());
  if w3 != 0 || w2 >= D {
    return None;
  } // quotient would exceed 128 bits
  let (q1, r) = div(w2, w1);
  let (q0, r) = div(r, w0);
  let mut q = ((q1 as u128) << 64) | q0 as u128; // |a*b| / 10^19, truncated
  let bump = match mode {
    Round::HalfEven => {
      let twice = r as u128 * 2;
      (twice > D as u128) | ((twice == D as u128) & (q & 1 == 1))
    }
    Round::Floor => neg & (r != 0), // toward -inf: grow magnitude if negative
  };
  q += bump as u128;
  if neg {
    if q > 1u128 << 127 {
      None
    } else {
      Some((q as i128).wrapping_neg())
    }
  } else {
    i128::try_from(q).ok()
  }
}

/// a * b for i128 values at fixed scale 19 (10^-19 units). None on overflow.
#[inline]
pub fn mul19(a: i128, b: i128) -> Option<i128> {
  mul19_impl(a, b, Round::HalfEven, div2by1)
}
/// Variant using hardware `div`; only competitive on CPUs with fast 64-bit divide.
#[cfg(all(target_arch = "x86_64", not(miri)))]
#[inline(never)]
pub fn mul19_hw(a: i128, b: i128) -> Option<i128> {
  mul19_impl(a, b, Round::HalfEven, div2by1_hw)
}
#[inline(never)]
pub fn mul19_floor(a: i128, b: i128) -> Option<i128> {
  mul19_impl(a, b, Round::Floor, div2by1)
}

// ---- Fast path: detect an exact whole-number operand (e.g. integer qty) ----
// 10^19 = 2^19 * 5^19. |b| is a whole number iff its low 19 bits are zero and
// (|b| >> 19) is divisible by 5^19. The divisibility test multiplies by the
// modular inverse of 5^19 (mod 2^128): no division needed, and the product is
// the exact quotient, i.e. the whole-number quantity itself.
const P5: u128 = 19073486328125; // 5^19;  10^19 = 2^19 * 5^19
const fn inv_mod_2_128(d: u128) -> u128 {
  // Newton: x = x*(2 - d*x), doubles correct bits
  let mut x = d;
  let mut i = 0;
  while i < 7 {
    x = x.wrapping_mul(2u128.wrapping_sub(d.wrapping_mul(x)));
    i += 1;
  }
  x
}
const INV5: u128 = inv_mod_2_128(P5);
const LIM5: u128 = u128::MAX / P5;

/// If |b| is an exact whole number at scale 19, return it as u64 (always fits).
#[inline(always)]
fn whole(b: i128) -> Option<u64> {
  let u = b.unsigned_abs();
  if u & ((1 << 19) - 1) != 0 {
    return None;
  } // must be divisible by 2^19: 1 AND
  let q = (u >> 19).wrapping_mul(INV5); // exact-division trick (Granlund–Montgomery)
  if q > LIM5 {
    return None;
  } // not divisible by 5^19
  Some(q as u64) // q = |b| / 10^19 < 2^127/10^19 < 2^64
}

#[inline]
pub fn mul19_fast(a: i128, b: i128) -> Option<i128> {
  if let Some(n) = whole(b) {
    // price * integer qty: exact, no division
    let u = a.unsigned_abs();
    let p0 = (u as u64 as u128) * n as u128; // 128x64 -> 192: two mulx
    let mid = ((u >> 64) as u64 as u128) * n as u128 + (p0 >> 64);
    if mid >> 64 != 0 {
      return None;
    }
    let m = (mid << 64) | (p0 as u64 as u128);
    let neg = (a < 0) ^ (b < 0);
    return if neg {
      if m > 1u128 << 127 {
        None
      } else {
        Some((m as i128).wrapping_neg())
      }
    } else {
      i128::try_from(m).ok()
    };
  }
  mul19(a, b)
}
