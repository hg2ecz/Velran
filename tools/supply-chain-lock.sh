#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
[ -f Cargo.lock ] || { echo 'cannot lock supply chain: Cargo.lock is missing' >&2; exit 1; }
[ -f SUPPLY-CHAIN-CAPABILITIES.txt ] || { echo 'cannot lock supply chain: capability policy is missing' >&2; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo 'cannot lock supply chain: cargo is required to validate Cargo.lock against manifests' >&2; exit 1; }
# Never seal an envelope around a lockfile that Cargo would rewrite. This is a
# security boundary: dependency resolution must already be explicit and stable.
cargo metadata --locked --format-version 1 >/dev/null || {
  echo 'cannot lock supply chain: Cargo.lock is stale for the workspace manifests' >&2
  echo 'run ./tools/refresh-lock.sh, review Cargo.lock, then retry' >&2
  exit 1
}
./tools/supply-chain-verify.sh --no-envelope
cargo_hash=$(sha256sum Cargo.lock | awk '{print $1}')
policy_hash=$(sha256sum SUPPLY-CHAIN-CAPABILITIES.txt | awk '{print $1}')
cat > VELRAN-SUPPLY-CHAIN.lock <<LOCK
format velran-supply-chain-v1
cargo-lock-sha256 $cargo_hash
capability-policy-sha256 $policy_hash
provenance crates.io-checksummed-only
capability-model direct-external-dependency-edge
LOCK
printf '%s\n' 'VELRAN-SUPPLY-CHAIN.lock updated; review capability changes before committing'
