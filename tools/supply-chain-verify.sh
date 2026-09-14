#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
MODE=${1:-verify}
POLICY=SUPPLY-CHAIN-CAPABILITIES.txt
[ -f Cargo.lock ] || { echo 'supply-chain verification refused: Cargo.lock is missing' >&2; exit 1; }
[ -f "$POLICY" ] || { echo "supply-chain verification refused: $POLICY is missing" >&2; exit 1; }

TMP=$(mktemp -d "${TMPDIR:-/tmp}/velran-supply.XXXXXX")
trap 'rm -rf "$TMP"' EXIT HUP INT TERM

# Workspace package names are trusted first-party path dependencies; external
# dependency edges must all appear in the capability review file.
for manifest in crates/*/Cargo.toml; do
  awk '/^name = / { gsub(/"/, "", $3); print $3; exit }' "$manifest"
done | LC_ALL=C sort -u > "$TMP/workspace"

: > "$TMP/edges-all"
for manifest in crates/*/Cargo.toml; do
  crate=$(awk '/^name = / { gsub(/"/, "", $3); print $3; exit }' "$manifest")
  awk -v crate="$crate" '
    /^\[/ { in_deps = ($0 == "[dependencies]" || $0 == "[dev-dependencies]" || $0 ~ /^\[target\..*\.dependencies\]$/); next }
    in_deps && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      line=$0; sub(/[[:space:]]*=.*/, "", line); print crate " " line
    }
  ' "$manifest" >> "$TMP/edges-all"
done
LC_ALL=C sort -u "$TMP/edges-all" | while read -r crate dep; do
  if ! grep -Fxq "$dep" "$TMP/workspace"; then printf '%s %s\n' "$crate" "$dep"; fi
done > "$TMP/edges"

awk '
  /^[[:space:]]*#/ || NF == 0 { next }
  NF != 4 { print "invalid policy row (expected: crate dependency capabilities reason-token): " $0 > "/dev/stderr"; bad=1; next }
  {
    n=split($3, caps, ",");
    for (i=1; i<=n; i++) if (caps[i] !~ /^(none|network|filesystem|environment|process|native|crypto|parser)$/) {
      print "unknown capability `" caps[i] "` in: " $0 > "/dev/stderr"; bad=1
    }
    print $1 " " $2
  }
  END { if (bad) exit 2 }
' "$POLICY" | LC_ALL=C sort -u > "$TMP/policy-edges"

diff -u "$TMP/edges" "$TMP/policy-edges" >/dev/null || {
  echo 'supply-chain capability policy does not match direct external dependency edges' >&2
  diff -u "$TMP/edges" "$TMP/policy-edges" >&2 || true
  exit 1
}

# Provenance rule for this release line: external Cargo packages must come from
# crates.io and carry Cargo.lock checksums. Git/alternate-registry provenance is
# rejected until explicitly designed and reviewed.
awk '
  /^\[\[package\]\]$/ { if (seen && external && (!cratesio || !checksum)) bad=1; seen=1; external=0; cratesio=0; checksum=0; next }
  /^source = / { external=1; if ($0 == "source = \"registry+https://github.com/rust-lang/crates.io-index\"") cratesio=1; next }
  /^checksum = "[0-9a-f]{64}"$/ { checksum=1; next }
  END { if (seen && external && (!cratesio || !checksum)) bad=1; exit bad }
' Cargo.lock || {
  echo 'supply-chain provenance verification failed: external packages must be checksummed crates.io packages; git/alternate registry sources are denied' >&2
  exit 1
}

# Lock and reviewed capability inventory are themselves release-state inputs.
if [ "$MODE" != "--no-envelope" ]; then
  [ -f VELRAN-SUPPLY-CHAIN.lock ] || { echo 'supply-chain verification refused: VELRAN-SUPPLY-CHAIN.lock is missing' >&2; exit 1; }
  cargo_hash=$(sha256sum Cargo.lock | awk '{print $1}')
  policy_hash=$(sha256sum "$POLICY" | awk '{print $1}')
  grep -Fxq 'format velran-supply-chain-v1' VELRAN-SUPPLY-CHAIN.lock || { echo 'invalid supply-chain lock format' >&2; exit 1; }
  grep -Fxq "cargo-lock-sha256 $cargo_hash" VELRAN-SUPPLY-CHAIN.lock || { echo 'VELRAN-SUPPLY-CHAIN.lock is stale for Cargo.lock' >&2; exit 1; }
  grep -Fxq "capability-policy-sha256 $policy_hash" VELRAN-SUPPLY-CHAIN.lock || { echo 'VELRAN-SUPPLY-CHAIN.lock is stale for capability policy' >&2; exit 1; }
  grep -Fxq 'provenance crates.io-checksummed-only' VELRAN-SUPPLY-CHAIN.lock || { echo 'unexpected supply-chain provenance policy' >&2; exit 1; }
  grep -Fxq 'capability-model direct-external-dependency-edge' VELRAN-SUPPLY-CHAIN.lock || { echo 'unexpected supply-chain capability model' >&2; exit 1; }
fi
echo 'supply-chain capability and provenance verification passed'
