//! Text in and out: the FIX/JSON hot path.
//!
//! Reference points: std and `itoa` on the raw i128 (formatting a plain
//! integer of the same size is the floor for formatting a decimal), and std's
//! f64 parsing and formatting (what a float-based system pays).

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use decimix::{Dec19, Round, dec};

fn bench_ascii(c: &mut Criterion) {
  let short = dec!(113.725);
  let long = dec!(-12345678901234567.1234567890123456789);
  let mut buf = [0u8; 64];

  let mut g = c.benchmark_group("format");
  g.bench_function("short", |b| {
    b.iter(|| black_box(short).write_ascii(black_box(&mut buf)))
  });
  g.bench_function("long", |b| {
    b.iter(|| black_box(long).write_ascii(black_box(&mut buf)))
  });
  g.bench_function("short_to_ascii", |b| {
    b.iter(|| black_box(short).to_ascii().len())
  });
  g.bench_function("short_dp4", |b| {
    b.iter(|| {
      black_box(short).write_ascii_dp(black_box(&mut buf), 4, Round::HalfEven)
    })
  });
  g.bench_function("long_dp4", |b| {
    b.iter(|| {
      black_box(long).write_ascii_dp(black_box(&mut buf), 4, Round::HalfEven)
    })
  });
  g.bench_function("ref/itoa_i128_raw_short", |b| {
    let mut ib = itoa::Buffer::new();
    b.iter(|| ib.format(black_box(short.to_raw())).len())
  });
  g.bench_function("ref/itoa_i128_raw_long", |b| {
    let mut ib = itoa::Buffer::new();
    b.iter(|| ib.format(black_box(long.to_raw())).len())
  });
  g.bench_function("ref/std_f64_short", |b| {
    use std::io::Write;
    b.iter(|| {
      let mut w = &mut buf[..];
      write!(w, "{}", black_box(113.725_f64)).unwrap();
    })
  });
  g.finish();

  let mut g = c.benchmark_group("parse");
  g.bench_function("short", |b| {
    b.iter(|| Dec19::from_ascii(black_box(b"113.725")))
  });
  g.bench_function("long", |b| {
    b.iter(|| {
      Dec19::from_ascii(black_box(b"-12345678901234567.1234567890123456789"))
    })
  });
  g.bench_function("integer", |b| {
    b.iter(|| Dec19::from_ascii(black_box(b"250")))
  });
  g.bench_function("ref/std_i128_short", |b| {
    b.iter(|| black_box("113725").parse::<i128>())
  });
  g.bench_function("ref/std_f64_short", |b| {
    b.iter(|| black_box("113.725").parse::<f64>())
  });
  g.finish();
}

criterion_group!(benches, bench_ascii);
criterion_main!(benches);
