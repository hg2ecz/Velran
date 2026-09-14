#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
fail(){ echo "native cache reuse integration: FAIL: $*" >&2; exit 1; }
T='crates/native-runtime/src/application_runtime/cache_reuse_tests.rs'
[ -f "$T" ] || fail 'cache reuse integration test missing'
grep -Fq 'second_bootstrap_uses_binary_cache_without_any_rustc_process' "$T" || fail 'warm-cache zero-rustc assertion missing'
grep -Fq 'assert_eq!(' "$T" || fail 'rustc invocation count equality assertion missing'
grep -Fq 'examples/typed-data-fastpaths/main.vrn' "$T" || fail 'representative native application missing'
if ! command -v cargo >/dev/null 2>&1 || ! command -v rustc >/dev/null 2>&1; then
    if [ "${VELRAN_REQUIRE_RUST_TOOLCHAIN:-0}" = "1" ]; then fail 'cargo/rustc required'; fi
    echo 'native cache reuse integration: SKIP toolchain execution (static invariants PASS)'
    exit 0
fi
VELRAN_RELEASE_CACHE_SMOKE=1 cargo test --locked -p native-runtime second_bootstrap_uses_binary_cache_without_any_rustc_process -- --nocapture
echo 'native cache reuse integration: PASS'
