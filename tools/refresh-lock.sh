#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

command -v cargo >/dev/null 2>&1 || { echo 'cargo is required' >&2; exit 1; }

# Dependency changes are explicit maintenance operations. This script is the
# only project helper that is allowed to resolve a new dependency graph.
cargo generate-lockfile
cargo metadata --locked --format-version 1 >/dev/null

echo 'Cargo.lock refreshed. Review the diff; if direct external dependency edges changed, update SUPPLY-CHAIN-CAPABILITIES.txt; then run ./tools/supply-chain-lock.sh, review VELRAN-SUPPLY-CHAIN.lock, and finally run ./verify.sh.'
