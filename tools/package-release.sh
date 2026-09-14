#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

if [ ! -f Cargo.lock ]; then
  echo 'release packaging refused: Cargo.lock is missing' >&2
  exit 1
fi

if [ ! -f RELEASE-MANIFEST.sha256 ]; then
  echo 'release packaging refused: generate RELEASE-MANIFEST.sha256 first' >&2
  exit 1
fi

command -v cargo >/dev/null 2>&1 || {
  echo 'release packaging refused: cargo is required to validate Cargo.lock against manifests' >&2
  exit 1
}
cargo metadata --locked --format-version 1 >/dev/null || {
  echo 'release packaging refused: Cargo.lock is stale for workspace manifests' >&2
  echo 'run ./tools/refresh-lock.sh, review Cargo.lock, then regenerate VELRAN-SUPPLY-CHAIN.lock' >&2
  exit 1
}

if ! sha256sum -c RELEASE-MANIFEST.sha256 >/dev/null; then
  echo 'release packaging refused: source manifest verification failed' >&2
  exit 1
fi

./tools/supply-chain-verify.sh

OUT=${1:-velran-v1.0.0-source.tgz}
TMP=$(mktemp "${TMPDIR:-/tmp}/velran-release.XXXXXX")
trap 'rm -f "$TMP"' EXIT HUP INT TERM
rm -f "$OUT"

# Source archives must not reset Rust inputs to the Unix epoch. Extracting an
# epoch-mtime source tree over a workspace that still has target/ can make
# Cargo reuse stale dependency metadata while recompiling newer dependants.
# Keep reproducible builds possible through an explicit, realistic
# SOURCE_DATE_EPOCH; otherwise stamp the archive with packaging time.
ARCHIVE_EPOCH=${SOURCE_DATE_EPOCH:-$(date +%s)}
case "$ARCHIVE_EPOCH" in
  ''|*[!0-9]*)
    echo 'release packaging refused: SOURCE_DATE_EPOCH must be an integer Unix timestamp' >&2
    exit 1
    ;;
esac
if [ "$ARCHIVE_EPOCH" -lt 946684800 ]; then
  echo 'release packaging refused: source archive timestamp must be >= 2000-01-01' >&2
  exit 1
fi

tar --sort=name --mtime="@$ARCHIVE_EPOCH" --owner=0 --group=0 --numeric-owner \
  --exclude='./.git' --exclude='./target' \
  -cf - . | gzip -n > "$TMP"
mv "$TMP" "$OUT"
trap - EXIT HUP INT TERM
sha256sum "$OUT"
