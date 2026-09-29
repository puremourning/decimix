# decimix

[![CI](https://github.com/puremourning/decimix/actions/workflows/ci.yml/badge.svg)](https://github.com/puremourning/decimix/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/decimix.svg)](https://crates.io/crates/decimix)
[![docs.rs](https://img.shields.io/docsrs/decimix)](https://docs.rs/decimix)
[![license](https://img.shields.io/crates/l/decimix.svg)](LICENSE)

Fixed-point decimal arithmetic: i128 at 19 decimal places

## Usage

```toml
[dependencies]
decimix = "0.1.0"
```

API documentation: <https://docs.rs/decimix>.

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
