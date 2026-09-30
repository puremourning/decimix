# CLAUDE.md

Guidance for Claude Code when working in this repository.

## Design

`docs/design-brief.md` holds the settled design decisions for `Dec19` and the
open questions. Treat the decisions as settled; raise the open questions rather
than deciding them silently. The multiply kernel in
`crates/decimix/src/kernel/mul.rs` came from that discussion; it has since been
extended to all six rounding modes and fixed for one overflow case (rounding up
past 128 bits), and is checked against a big-integer oracle in
`tests/mul_oracle.rs`.

## Layout

Cargo workspace. Crates live under `crates/<name>`; the published crate is
`crates/decimix`. Shared package metadata (version, edition,
rust-version, license, authors, repository) is in `[workspace.package]` in the
root `Cargo.toml` and inherited with `x.workspace = true`. Shared dependency
versions go in `[workspace.dependencies]`.

## Commands

- Build / test: `cargo build --workspace --all-targets`, `cargo test --workspace`
- Lint: `cargo clippy --workspace --all-targets -- -Dwarnings`
- Format: `cargo fmt --all`. Formatting uses nightly rustfmt with the unstable
  options in `.rustfmt.toml` (2-space indent, 80 columns); CI checks it with
  `--unstable-features --error-on-unformatted`. `rust-toolchain.toml` pins
  nightly for local work.
- Docs: `cargo doc --workspace --no-deps` (CI sets `RUSTDOCFLAGS=-Dwarnings`)
- Miri: `cargo miri test --workspace`

## CI and releases

`.github/workflows/ci.yml` and `release.yml` are thin callers of the reusable
workflows in <https://github.com/puremourning/rust-ci> (`@v1`). CI tests the
MSRV (1.98) plus stable, beta and nightly, and runs rustfmt, clippy, docs
and (if enabled) miri. Project-specific checks go in extra jobs alongside the
`ci` job.

System dependencies (e.g. Cap'n Proto, apt packages) go in a local composite
action at `.github/actions/setup/action.yml`, which every shared job runs after
installing the toolchain if the file exists:

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

Releases are cut with cargo-release via the manual **Release** workflow; config
is `[workspace.metadata.release]` in `Cargo.toml`. Record API changes in
`CHANGELOG.md` under `[Unreleased]`.
