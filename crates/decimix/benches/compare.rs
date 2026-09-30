//! decimix against the alternatives named in the design brief, on the same
//! values and operations:
//! - `primitive_fixed_point_decimal`: the same representation (i128 at a
//!   fixed scale of 19), whose `Round` mode is half-away-from-zero, so
//!   decimix uses `HalfAwayFromZero` here;
//! - `fpdec`: a floating decimal (i128 coefficient plus a per-value scale,
//!   at most 18 places), whose products are exact and grow in scale;
//! - `f64`, what float-based systems use.

use std::hint::black_box;
use std::str::FromStr;

use criterion::{Criterion, criterion_group, criterion_main};
use decimix::{Dec19, Round};
use fpdec::Decimal;
use primitive_fixed_point_decimal::{ConstScaleFpdec, Rounding};

type Pfpd = ConstScaleFpdec<i128, 19>;

const PX: &str = "113.725";
const QTY: &str = "1234.5678901";
const LOTS: &str = "250";

fn bench_compare(c: &mut Criterion) {
  let mode = Round::HalfAwayFromZero;
  let (d_px, d_qty, d_lots) = (
    Dec19::from_str(PX).unwrap(),
    Dec19::from_str(QTY).unwrap(),
    Dec19::from_str(LOTS).unwrap(),
  );
  let (p_px, p_qty, p_lots) = (
    Pfpd::from_str(PX).unwrap(),
    Pfpd::from_str(QTY).unwrap(),
    Pfpd::from_str(LOTS).unwrap(),
  );
  let (f_px, f_qty, f_lots) = (
    Decimal::from_str(PX).unwrap(),
    Decimal::from_str(QTY).unwrap(),
    Decimal::from_str(LOTS).unwrap(),
  );
  let (x_px, x_qty, x_lots) = (113.725_f64, 1234.5678901_f64, 250.0_f64);

  let mut g = c.benchmark_group("add");
  g.bench_function("decimix", |b| {
    b.iter(|| black_box(d_px) + black_box(d_qty))
  });
  g.bench_function("pfpd", |b| b.iter(|| black_box(p_px) + black_box(p_qty)));
  g.bench_function("fpdec", |b| b.iter(|| black_box(f_px) + black_box(f_qty)));
  g.bench_function("f64", |b| b.iter(|| black_box(x_px) + black_box(x_qty)));
  g.finish();

  let mut g = c.benchmark_group("mul");
  g.bench_function("decimix", |b| {
    b.iter(|| black_box(d_px).checked_mul(black_box(d_qty), mode))
  });
  g.bench_function("pfpd", |b| {
    b.iter(|| {
      black_box(p_px)
        .checked_mul_ext::<_, 19, 19>(black_box(p_qty), Rounding::Round)
    })
  });
  g.bench_function("fpdec", |b| b.iter(|| black_box(f_px) * black_box(f_qty)));
  g.bench_function("f64", |b| b.iter(|| black_box(x_px) * black_box(x_qty)));
  g.finish();

  let mut g = c.benchmark_group("mul_whole_qty");
  g.bench_function("decimix", |b| {
    b.iter(|| black_box(d_px).checked_mul(black_box(d_lots), mode))
  });
  g.bench_function("pfpd", |b| {
    b.iter(|| {
      black_box(p_px)
        .checked_mul_ext::<_, 19, 19>(black_box(p_lots), Rounding::Round)
    })
  });
  g.bench_function("fpdec", |b| b.iter(|| black_box(f_px) * black_box(f_lots)));
  g.bench_function("f64", |b| b.iter(|| black_box(x_px) * black_box(x_lots)));
  g.finish();

  let mut g = c.benchmark_group("div");
  g.bench_function("decimix", |b| {
    b.iter(|| black_box(d_px).checked_div(black_box(d_qty), mode))
  });
  g.bench_function("pfpd", |b| {
    b.iter(|| {
      black_box(p_px)
        .checked_div_ext::<_, 19, 19>(black_box(p_qty), Rounding::Round)
    })
  });
  g.bench_function("fpdec", |b| b.iter(|| black_box(f_px) / black_box(f_qty)));
  g.bench_function("f64", |b| b.iter(|| black_box(x_px) / black_box(x_qty)));
  g.finish();

  let mut g = c.benchmark_group("parse");
  g.bench_function("decimix", |b| b.iter(|| Dec19::from_str(black_box(PX))));
  g.bench_function("pfpd", |b| b.iter(|| Pfpd::from_str(black_box(PX))));
  g.bench_function("fpdec", |b| b.iter(|| Decimal::from_str(black_box(PX))));
  g.bench_function("f64", |b| b.iter(|| f64::from_str(black_box(PX))));
  g.finish();

  let mut g = c.benchmark_group("format");
  g.bench_function("decimix", |b| b.iter(|| black_box(d_px).to_string()));
  g.bench_function("decimix_write_ascii", |b| {
    let mut buf = [0u8; 64];
    b.iter(|| black_box(d_px).write_ascii(black_box(&mut buf)))
  });
  g.bench_function("pfpd", |b| b.iter(|| black_box(p_px).to_string()));
  g.bench_function("fpdec", |b| b.iter(|| black_box(f_px).to_string()));
  g.bench_function("f64", |b| b.iter(|| black_box(x_px).to_string()));
  g.finish();
}

criterion_group!(benches, bench_compare);
criterion_main!(benches);
