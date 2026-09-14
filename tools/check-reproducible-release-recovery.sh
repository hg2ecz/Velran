#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
fail(){ echo "reproducible release/recovery gate: FAIL: $*" >&2; exit 1; }
C='crates/native-build/src/build_cache/cleanup.rs'
[ -f "$C" ] || fail 'abandoned cache staging cleanup missing'
grep -Fq '.rustc-identity-v1.' "$C" || fail 'rustc identity partial-file recovery missing'
grep -Fq '.work-' "$C" || fail 'direct-rustc work directory recovery missing'
grep -Fq 'key.len() == 64' "$C" || fail 'cache staging cleanup name validation missing'
grep -Fq 'Path::new("/proc")' "$C" || fail 'live-process protection missing on Linux'

if ! command -v cargo >/dev/null 2>&1 || ! command -v rustc >/dev/null 2>&1; then
    if [ "${VELRAN_REQUIRE_RUST_TOOLCHAIN:-0}" = "1" ]; then fail 'cargo/rustc required'; fi
    echo 'reproducible release/recovery gate: SKIP toolchain execution (static invariants PASS)'
    exit 0
fi
case "${SOURCE_DATE_EPOCH:-}" in
  ''|*[!0-9]*) fail 'SOURCE_DATE_EPOCH must be explicitly set to an integer for reproducible release verification' ;;
esac
[ "$SOURCE_DATE_EPOCH" -ge 946684800 ] || fail 'SOURCE_DATE_EPOCH must be >= 2000-01-01'

cargo test --locked -p native-build removes_only_abandoned_known_staging_names
cargo build --locked --release -p velran-server
build_id_1=$(target/release/velran-server --print-build-id)
build_id_2=$(target/release/velran-server --print-build-id)
case "$build_id_1" in ''|*[!0-9a-f]* ) fail 'server build id is not lowercase hexadecimal' ;; esac
[ "${#build_id_1}" -eq 64 ] || fail 'server build id must be 64 hex characters'
[ "$build_id_1" = "$build_id_2" ] || fail 'server build id is not deterministic within the exact release binary'

./tools/release-manifest.sh > RELEASE-MANIFEST.sha256
sha256sum -c RELEASE-MANIFEST.sha256 >/dev/null
one=$(mktemp "${TMPDIR:-/tmp}/velran-release-one.XXXXXX.tgz")
two=$(mktemp "${TMPDIR:-/tmp}/velran-release-two.XXXXXX.tgz")
trap 'rm -f "$one" "$two"' EXIT HUP INT TERM
SOURCE_DATE_EPOCH="$SOURCE_DATE_EPOCH" ./tools/package-release.sh "$one" >/dev/null
SOURCE_DATE_EPOCH="$SOURCE_DATE_EPOCH" ./tools/package-release.sh "$two" >/dev/null
hash_one=$(sha256sum "$one" | awk '{print $1}')
hash_two=$(sha256sum "$two" | awk '{print $1}')
[ "$hash_one" = "$hash_two" ] || fail 'same-tree source packages are not byte-for-byte reproducible'
echo "reproducible release/recovery gate: PASS build_id=$build_id_1 source_sha256=$hash_one"
