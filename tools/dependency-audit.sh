#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

[ -f Cargo.lock ] || { echo 'Cargo.lock is missing; run ./tools/refresh-lock.sh first' >&2; exit 1; }

./tools/supply-chain-verify.sh

cargo metadata --locked --format-version 1 >/dev/null
cargo tree --locked --duplicates || true

if command -v cargo-audit >/dev/null 2>&1; then
    cargo audit --deny warnings
else
    echo 'NOTE: cargo-audit is not installed; RustSec audit was not executed.' >&2
    echo 'Install cargo-audit in CI/release tooling and rerun this command.' >&2
fi
