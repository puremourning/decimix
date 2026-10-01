# AGENTS.md

Guidance for coding agents (and people) working in this repository: the
layout, the commands, and the house rules established while building it.

## Design

- `docs/design-brief.md` holds the settled design decisions for `Dec19` and
  the open questions. Treat the decisions as settled; **raise open questions
  rather than deciding them silently**. API and format decisions (names,
  semantics, wire and storage formats) are the maintainer's call: propose
  with a recommendation, don't just pick one.
- `docs/TODO.md` lists what's decided or noted but not done. Keep it current:
  add parked items, remove finished ones.
- Design proposals go in `docs/` as their own documents for review (e.g.
  `docs/compact-columns.md`).
- The multiply kernel in `crates/decimix/src/kernel/mul.rs` came from the
  design discussion. Its header lists every change since; keep that list up
  to date.

## API rules

The crate should be hard to misuse. Keep to these unless the maintainer
agrees otherwise:

- **Operators only for exact operations:** `+`, `-`, unary `-`, comparisons,
  and `*` by an integer. Anything that can round (decimal × decimal, every
  division, rounding to steps or places) is a method taking a mandatory
  `Round`. No default rounding mode and no hidden context.
- **Overflow:** operators panic, in release builds too. Provide `checked_*`
  (returns `Option`) and `saturating_*` forms. No `wrapping_*` or
  `overflowing_*`: a wrapped price is never wanted.
- **Floats only at the edges:** no `From`/`TryFrom`/`Into` with `f32`/`f64`,
  no `as_f64`, no `num-traits` conversion traits. The only crossings are
  `to_f64_lossy` and `from_f64_lossy(x, step, Round)`, which must stay
  loudly named and bannable with the `clippy.toml` snippet in the crate docs
  (checked by `ci/check-float-lint.sh`). Domain newtypes have no f64
  methods.
- **One representation per value**, so derived `Eq`/`Ord`/`Hash` are correct
  and bytes are canonical.
- **Errors are reported in a fixed order:** not a number (`Invalid`), then
  out of range, then too precise. A value that is both out of range and too
  precise reports out of range.
- **Names:** follow std where std has the concept (`div_euclid`,
  `rem_euclid`, `to_le_bytes`, `checked_*`); `to_` returns an owned value,
  `as_` borrows. Prefer plain English over jargon for public names
  (`SMALLEST_STEP`, `DECIMAL_PLACES`), with the jargon as `#[doc(alias)]`.
  Don't reuse a word that means something else in this API (e.g. "floor" is
  a `Round` mode).
- **Domain types** (`decimix-finance`) follow the FIX 5 datatype names
  (`Price`, `Qty`, `Amt`, `Percentage`), plus `DeltaQty` for signed
  quantities. Products are defined per kind of value (the `Quantity` trait),
  not per sign.
- Record API changes in `CHANGELOG.md` under `[Unreleased]`.

## Code rules

- **Plain-English maths.** Every non-obvious arithmetic step gets a comment
  that says what it computes and why it's correct, with a small worked
  example where it helps. Named techniques ("Möller–Granlund", "SWAR") are
  pointers for further reading, never the explanation. Write constants out
  the first time (10¹⁹ = 10,000,000,000,000,000,000). Every "can't overflow"
  claim says why. The same goes for rustdoc: explain rounding, ranges and
  steps without assuming a maths degree.
- **No target-specific code** by default. Escalate only when a benchmark
  justifies it, in this order: portable scalar/SWAR code, then SIMD via
  `core`, then inline asm, always keeping the portable version as the
  fallback and testing both against the same oracle.
- **`unsafe`:** the crate has `#![deny(unsafe_code)]`. The one exception is
  `AsciiBuf::as_str`, allowed locally with a `// SAFETY:` comment and a
  `debug_assert!` of its invariant. Any new `unsafe` needs the same:
  measured benefit, local `#[allow]`, SAFETY comment listing every way the
  invariant is upheld, and a debug check; and the maintainer's agreement.
- `#![no_std]`; no allocation on hot paths (text in and out works on caller
  buffers or fixed stack buffers).
- The types are `#[must_use]`; don't also put `#[must_use]` on functions
  returning them (clippy's `double_must_use`), but do on functions
  returning `Option`, `Result` or plain values.

## Testing rules

- **Oracles:** every arithmetic, rounding, parsing, formatting and
  conversion path is checked against an exact big-integer oracle
  (`proptest` + `num-bigint`), in all six rounding modes, for both `Dec19`
  and `UDec19`. Build oracles *differently* from the implementation (e.g.
  floor the signed value, then pick a neighbour; see
  `tests/common/mod.rs::round_div`), so they can't share its mistakes.
  Where a test necessarily relies on the same foundation as the code
  (e.g. std's float formatting), add an independent check if one exists.
- Use the edge-biased generators in `tests/common/mod.rs` (`value`,
  `uvalue`): powers of ten and neighbours, exact halves, whole numbers,
  limits. Search directly for rare cases random inputs can't reach (the
  quotient-overflow and huge-whole-number bugs were found that way).
- **Prove new tests have teeth:** plant a bug in the code, confirm the test
  fails, then restore. Commit the resulting `*.proptest-regressions` files;
  they are useful edge cases.
- **Things that must not compile** are `compile_fail` doc tests. Because
  those pass on *any* error, check each one fails for the intended reason
  (compile the snippet and read the error).
- Exhaustive tests at small widths where the logic is width-independent
  (the rounding decision is tested on every 8-bit case).
- **Miri** runs with few proptest cases (8, via `common::config`); keep new
  suites Miri-friendly and skip wall-clock timing assertions under Miri.
- The README's example is compiled and run as a doc test; keep it working.
- Domain types in `decimix-finance` get light tests only (their future is
  undecided); the base types carry the thorough ones.

## Performance rules

- Benchmark before and after any change to a hot path (criterion
  baselines: `--save-baseline` / `--baseline`), and report the numbers.
- Make benchmarks honest: pass values through `black_box`, and don't let a
  constant (e.g. a tick size captured by a closure) be folded where real
  code would have a runtime value. Compare against the reference points in
  the benches (std, `itoa`, `primitive_fixed_point_decimal`, `fpdec`, f64).
- If a wrapper looks slower than its kernel, compare the generated code
  before chasing it; code layout can move timings by a few nanoseconds.

## Workflow

- Work in phases; commit each on a branch, never directly on `main`. The
  maintainer pushes; don't push unless asked.
- Before committing, run everything in **Commands** below, including the
  all-features and no-features variants and Miri on changed suites.
- Tell the maintainer what was verified and what wasn't (e.g. "ran locally
  but not in CI").

## Layout

Cargo workspace. Crates live under `crates/<name>`: `crates/decimix` is the
published crate, `crates/decimix-finance` the domain types (not published).
`crates/capnp-decimix` is the Cap'n Proto encoding (not yet published), and
`crates/capnp-decimix-tests` holds its tests: it compiles a test schema the
way a user's crate would, through `import_path()` and `SCHEMA_ID`.
Shared package metadata (version, edition, rust-version, license, authors,
repository) is in `[workspace.package]` in the root `Cargo.toml` and
inherited with `x.workspace = true`. Shared dependency versions go in
`[workspace.dependencies]`.

Outside the workspace: `fuzz/` (cargo-fuzz targets, each checked against an
oracle) and `ci/float-lint/` (a fixture proving the recommended `clippy.toml`
bans the f64 methods).

## Commands

- Build / test: `cargo build --workspace --all-targets`,
  `cargo test --workspace` and `cargo test --workspace --all-features`
- Lint: `cargo clippy --workspace --all-targets -- -Dwarnings`, and again
  with `--all-features`
- Format: `cargo fmt --all`. Formatting uses nightly rustfmt with the
  unstable options in `.rustfmt.toml` (2-space indent, 80 columns); CI
  checks it with `--unstable-features --error-on-unformatted`.
  `rust-toolchain.toml` pins nightly for local work.
- Docs: `RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps --all-features`
- Miri: `cargo miri test --workspace --all-features`
- Float lint: `ci/check-float-lint.sh`
- Benchmarks: `cargo bench --bench <mul|arith|ascii|ieee|compare>`
- Fuzzing (needs `cargo install cargo-fuzz`):
  `cd fuzz && cargo fuzz run <parse|arith|f64> -- -max_total_time=60`

## CI and releases

`.github/workflows/ci.yml` and `release.yml` are thin callers of the reusable
workflows in <https://github.com/puremourning/rust-ci> (`@v1`). CI tests the
MSRV (1.98) plus stable, beta and nightly, and runs rustfmt, clippy, docs and
Miri, all with `--all-features`. Project-specific jobs sit alongside the `ci`
job: `no-features` (the default build) and `float-lint`. The separate
`fuzz.yml` workflow fuzzes on pushes to `main`, weekly, and on demand.

System dependencies (e.g. Cap'n Proto, apt packages) go in a local composite
action at `.github/actions/setup/action.yml`, which every shared job runs after
installing the toolchain if the file exists. Here it builds `capnp` from the
`newtype-v2` branch of the Cap'n Proto fork, which the capnp crates' schemas
need; a release would look like:

```yaml
name: Setup
description: Project setup hook
runs:
  using: composite
  steps:
    - uses: puremourning/rust-ci/setup-capnproto@v1
      with:
        version: "1.3.0"
```

Releases are cut with cargo-release via the manual **Release** workflow;
config is `[workspace.metadata.release]` in `Cargo.toml`.
