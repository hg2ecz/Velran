#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
B="$ROOT/crates/native-runtime/src/application_runtime/bootstrap.rs"
E="$ROOT/crates/native-runtime/src/application_runtime/engine.rs"
I="$ROOT/crates/native-runtime/src/application_runtime/toolchain_identity_cache.rs"
fail(){ echo "native binary cache fastpath gate: FAIL: $*" >&2; exit 1; }
[ -f "$I" ] || fail 'toolchain identity cache module missing'
grep -Fq 'toolchain_identity_cache::load(cache.root(), &probe_key)' "$B" || fail 'cached rustc identity is not loaded before probing'
grep -Fq 'None => {' "$B" || fail 'rustc identity cache miss branch missing'
grep -Fq 'let identity = rustc_identity(' "$B" || fail 'rustc probe missing from cache-miss branch'
grep -Fq '&rustc,' "$B" || fail 'rustc probe does not use resolved compiler path'
grep -Fq 'toolchain_identity_cache::store(cache.root(), &probe_key, &identity)' "$B" || fail 'rustc identity is not persisted'
grep -Fq 'native_binary_cache_hit' "$E" || fail 'binary cache hit event missing'
grep -Fq 'native_binary_cache_miss_compiled' "$E" || fail 'binary cache miss/compile event missing'
grep -Fq 'rustc_modified_nanos' "$I" || fail 'rustc binary metadata identity is incomplete'
grep -Fq 'SERVER_BUILD_CONTRACT_VERSION' "$ROOT/crates/server/src/build_identity.rs" || fail 'server build contract version missing'
grep -Fq 'server-build-contract-2-native-cache-fastpath' "$ROOT/crates/server/src/build_identity.rs" || fail 'server build contract was not bumped for cache fastpath'
echo 'native binary cache fastpath gate: PASS'
