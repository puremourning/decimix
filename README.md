# decimix

[![CI](https://github.com/puremourning/decimix/actions/workflows/ci.yml/badge.svg)](https://github.com/puremourning/decimix/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/decimix.svg)](https://crates.io/crates/decimix)
[![docs.rs](https://img.shields.io/docsrs/decimix)](https://docs.rs/decimix)
[![license](https://img.shields.io/crates/l/decimix.svg)](LICENSE)

Fixed-point decimal arithmetic for trading systems: fast, exact, and hard to
misuse.

`Dec19` is a signed number with exactly **19 decimal places** (not SQL
`DECIMAL(19)`, which means 19 digits in total), stored as an `i128`: a range
of about ±1.7 × 10¹⁹ with 38 significant digits. `UDec19` is the unsigned
version. There is one representation per value, so comparing and hashing are
plain integer operations, and adding costs the same as for an `i128`.

## Usage

```toml
[dependencies]
decimix = "0.1.0"
```

```rust
use decimix::{Dec19, Round, dec};

const TICK: Dec19 = dec!(0.005); // checked at compile time, never via f64
let bid = dec!(113.725);
let ask = dec!(113.74);

// Exact operations are operators.
let spread = ask - bid;
assert_eq!(spread * 2, dec!(0.03));

// Anything that can round is a method, and you say how to round.
let mid = (bid + ask).div_int(2, Round::HalfEven);
assert_eq!(mid.round_to(TICK, Round::Floor), dec!(113.73));
let notional = bid.mul(dec!(1234.5678901), Round::HalfEven);

// Text in and out is exact, and fast enough for FIX.
let mut buf = [0u8; Dec19::MAX_ASCII_LEN];
let n = mid.write_ascii(&mut buf).unwrap();
assert_eq!(Dec19::from_ascii(&buf[..n]), Ok(mid));
```

- **No hidden rounding.** `*` between two decimals and `/` don't exist:
  `mul`, `div`, `div_int`, `round_to` and friends take a mandatory `Round`
  (six modes). Overflow panics in operators; `checked_*` and `saturating_*`
  don't.
- **Floats only at the edges.** No `From<f64>` or `as_f64`. Two loudly named
  methods, `to_f64_lossy` and `from_f64_lossy(x, step, round)`, talk to
  systems that use doubles, and core crates can ban them with Clippy (see the
  crate docs for the `clippy.toml` snippet).
- **Exact slow path.** With the `bigdecimal` feature, `to_big()` converts
  exactly for maths this crate doesn't do, and `from_big(x, round)` comes
  back.
- **Sums of products** (`ProductSum`) are kept exactly and rounded once.
- **Domain types.** `newtype!` defines types like `Price` and `Qty` that
  can't be mixed by accident; the `decimix-finance` crate has a set named
  after the FIX datatypes.

API documentation: <https://docs.rs/decimix>. The design decisions are in
[`docs/design-brief.md`](docs/design-brief.md).

## Performance

Rough figures on one machine (Apple Silicon); see `crates/decimix/benches`,
including `compare.rs`, which runs the same operations with
`primitive_fixed_point_decimal`, `fpdec` and `f64`.

| Operation | ns |
|---|---|
| add, compare | < 1 |
| price × quantity, rounded | 6.5–10 |
| price × whole quantity | 2–2.6 |
| divide | 30 (10 by a whole number) |
| parse `113.725` | 8.7 |
| format `113.725` (`write_ascii`) | 19 |
| `to_f64_lossy` | 10 |

## Testing

Arithmetic, parsing, formatting and conversions are checked against exact
big-integer oracles in every rounding mode (`proptest`), under Miri, and by
coverage-guided fuzzing (`fuzz/`, run in CI).

## Minimum supported Rust version

1.98. Raising it is not considered a breaking change, but is noted in the
[changelog](CHANGELOG.md).

## Releasing

`decimix` follows [SemVer](https://semver.org). While the crate is on
`0.x`, breaking changes bump the minor version (`0.1.0` → `0.2.0`) and
backward-compatible changes bump the patch version (`0.1.0` → `0.1.1`).

Releases are cut by the [`Release`](.github/workflows/release.yml) workflow,
which calls the shared one in
[puremourning/rust-ci](https://github.com/puremourning/rust-ci). To publish:

1. Make sure `main` is green on CI and contains everything you want in the
   release.
2. From the GitHub **Actions** tab, run **Release** via *Run workflow* and
   enter the new version (e.g. `0.2.0`, no `v` prefix) or a bump level
   (`patch`, `minor`, `major`). Tick *dry-run* to see what would happen.

The workflow runs the tests, then `cargo release`: it bumps the version in
[`Cargo.toml`](Cargo.toml) and the install snippet above, verifies with
`cargo publish --dry-run`, commits `Release vX.Y.Z`, tags `vX.Y.Z`, pushes,
publishes to crates.io and creates a GitHub release with generated notes.

### Required setup

- `CARGO_REGISTRY_TOKEN` repository secret — a crates.io API token from
  <https://crates.io/me>, scoped to `publish-new` for the first release and
  `publish-update` for `decimix` afterwards.
- The default `GITHUB_TOKEN` is enough for the commit/tag push and release
  creation, provided `main` doesn't have branch protection that blocks
  pushes from `github-actions[bot]`.

## License

Licensed under the [MIT License](LICENSE).
