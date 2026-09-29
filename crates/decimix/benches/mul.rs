use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use decimix::kernel::mul;

const D: i128 = 10_000_000_000_000_000_000;

fn bench_mul(c: &mut Criterion) {
  // 113.725 * 1234.5678901: a price times a fractional quantity.
  let px = 113_725 * D / 1_000;
  let qty = 12_345_678_901 * D / 10_000_000;
  // A whole-number quantity, which takes the fast path.
  let lots = 250 * D;

  let mut g = c.benchmark_group("mul");
  g.bench_function("mul19", |b| {
    b.iter(|| mul::mul19(black_box(px), black_box(qty)))
  });
  g.bench_function("mul19_floor", |b| {
    b.iter(|| mul::mul19_floor(black_box(px), black_box(qty)))
  });
  g.bench_function("mul19_fast/hit", |b| {
    b.iter(|| mul::mul19_fast(black_box(px), black_box(lots)))
  });
  g.bench_function("mul19_fast/miss", |b| {
    b.iter(|| mul::mul19_fast(black_box(px), black_box(qty)))
  });
  g.bench_function("i128_mul", |b| {
    b.iter(|| black_box(px).wrapping_mul(black_box(qty)))
  });
  g.finish();
}

criterion_group!(benches, bench_mul);
criterion_main!(benches);
