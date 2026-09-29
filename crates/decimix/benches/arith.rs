//! Multiplication, division and sums of products through the public API.
//! Compare `mul/api` with `mul/mul19` in the `mul` bench to see the cost of
//! the typed wrapper over the raw kernel.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use decimix::{ProductSum, Round, dec, udec};

fn bench_arith(c: &mut Criterion) {
  let px = dec!(113.725);
  let qty = dec!(1234.5678901);
  let lots = dec!(250);
  let uqty = udec!(250);
  let mode = Round::HalfEven;

  let mut g = c.benchmark_group("mul");
  g.bench_function("api", |b| {
    b.iter(|| black_box(px).mul(black_box(qty), black_box(mode)))
  });
  g.bench_function("api_const_mode", |b| {
    b.iter(|| black_box(px).mul(black_box(qty), Round::HalfEven))
  });
  g.bench_function("api_checked", |b| {
    b.iter(|| black_box(px).checked_mul(black_box(qty), Round::HalfEven))
  });
  g.bench_function("kernel_same_binary", |b| {
    let (x, y) = (px.to_raw(), qty.to_raw());
    b.iter(|| decimix::kernel::mul::mul19_fast(black_box(x), black_box(y)))
  });
  g.bench_function("api_whole", |b| {
    b.iter(|| black_box(px).mul(black_box(lots), black_box(mode)))
  });
  g.bench_function("api_whole_first", |b| {
    b.iter(|| black_box(lots).mul(black_box(px), black_box(mode)))
  });
  g.bench_function("api_unsigned_qty", |b| {
    b.iter(|| black_box(px).mul(black_box(uqty), black_box(mode)))
  });
  g.finish();

  let mut g = c.benchmark_group("div");
  g.bench_function("general", |b| {
    b.iter(|| black_box(px).div(black_box(qty), black_box(mode)))
  });
  g.bench_function("by_whole", |b| {
    b.iter(|| black_box(px).div(black_box(lots), black_box(mode)))
  });
  g.bench_function("div_int", |b| {
    b.iter(|| black_box(px).div_int(black_box(250), black_box(mode)))
  });
  g.bench_function("round_to_tick", |b| {
    let tick = dec!(0.005);
    b.iter(|| black_box(qty).round_to(black_box(tick), black_box(mode)))
  });
  g.finish();

  let fills = [
    (dec!(113.725), dec!(100)),
    (dec!(113.73), dec!(250.5)),
    (dec!(113.74), dec!(50)),
    (dec!(113.72), dec!(1234.5678901)),
  ];
  let mut g = c.benchmark_group("product_sum");
  g.bench_function("4_fills_finish", |b| {
    b.iter(|| {
      let mut acc = ProductSum::new();
      for (p, q) in black_box(&fills) {
        acc.add(*p, *q);
      }
      acc.finish(mode)
    })
  });
  g.bench_function("4_fills_rounding_each", |b| {
    b.iter(|| {
      let mut total = dec!(0);
      for (p, q) in black_box(&fills) {
        total += p.mul(*q, mode);
      }
      total
    })
  });
  g.bench_function("4_fills_vwap", |b| {
    let total_qty = dec!(1635.0678901);
    b.iter(|| {
      let mut acc = ProductSum::new();
      for (p, q) in black_box(&fills) {
        acc.add(*p, *q);
      }
      acc.div(black_box(total_qty), mode)
    })
  });
  g.finish();
}

criterion_group!(benches, bench_arith);
criterion_main!(benches);
