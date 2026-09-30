#!/bin/sh
# Checks that the clippy.toml snippet recommended in the decimix crate docs
# bans all four f64 methods, and that #[allow] opts back in.
set -eu
cd "$(dirname "$0")/float-lint"

echo "clippy with only an #[allow]ed use: must pass"
cargo clippy --quiet -- -Dwarnings

echo "clippy with banned uses: must fail, naming each method"
if out=$(cargo clippy --quiet --features violate -- -Dwarnings 2>&1); then
  echo "error: clippy accepted the banned f64 methods" >&2
  exit 1
fi
for method in \
  decimix::Dec19::to_f64_lossy \
  decimix::Dec19::from_f64_lossy \
  decimix::UDec19::to_f64_lossy \
  decimix::UDec19::from_f64_lossy
do
  if ! printf '%s\n' "$out" | grep -q "disallowed method \`$method\`"; then
    echo "error: clippy did not flag $method" >&2
    printf '%s\n' "$out" >&2
    exit 1
  fi
  echo "  flagged $method"
done
echo "ok"
