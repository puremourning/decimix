//! The f64 boundary. Reference points: std parsing and formatting f64,
//! which the slow paths use and which a float-based system pays anyway.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use decimix::{Dec19, Round, dec};

fn bench_ieee(c: &mut Criterion) {
  let short = dec!(113.725);
  let long = dec!(12345678901.1234567890123456789);
  let tick = dec!(0.005);
  let mode = Round::HalfEven;

  let mut g = c.benchmark_group("to_f64");
  g.bench_function("price", |b| b.iter(|| black_box(short).to_f64_lossy()));
  g.bench_function("long_slow_path", |b| {
    b.iter(|| black_box(long).to_f64_lossy())
  });
  g.bench_function("ref/std_parse_f64", |b| {
    b.iter(|| black_box("113.725").parse::<f64>())
  });
  g.finish();

  let mut g = c.benchmark_group("from_f64");
  g.bench_function("price_to_tick", |b| {
    b.iter(|| Dec19::from_f64_lossy(black_box(113.725), black_box(tick), mode))
  });
  g.bench_function("price_all_digits", |b| {
    b.iter(|| {
      Dec19::from_f64_lossy(black_box(113.725), Dec19::SMALLEST_STEP, mode)
    })
  });
  g.bench_function("noisy", |b| {
    b.iter(|| {
      Dec19::from_f64_lossy(black_box(0.1 + 0.2), black_box(tick), mode)
    })
  });
  g.bench_function("ref/std_format_f64", |b| {
    use std::io::Write;
    let mut buf = [0u8; 64];
    b.iter(|| {
      let mut w = &mut buf[..];
      write!(w, "{}", black_box(113.725_f64)).unwrap();
    })
  });
  g.finish();
}

criterion_group!(benches, bench_ieee);
criterion_main!(benches);
