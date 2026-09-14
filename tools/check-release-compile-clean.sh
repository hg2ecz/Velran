#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
fail(){ echo "release compile-clean gate: FAIL: $*" >&2; exit 1; }

# Static invariants are always checked, even on audit hosts without a Rust toolchain.
grep -Fq 'release_native_positive_smoke_compiles_generated_cdylibs' crates/native-build/src/incremental_build/tests/release_native_smoke.rs || fail 'native direct-rustc release smoke test missing'
grep -Fq 'VELRAN_RELEASE_NATIVE_SMOKE' crates/native-build/src/incremental_build/tests/release_native_smoke.rs || fail 'native release smoke opt-in missing'
grep -Fq 'examples/pure-fn-tagged-sum/main.vrn' crates/native-build/src/incremental_build/tests/release_native_smoke.rs || fail 'sum-type generated shard smoke coverage missing'
grep -Fq 'examples/pure-fn-typed-struct/main.vrn' crates/native-build/src/incremental_build/tests/release_native_smoke.rs || fail 'typed-struct generated shard smoke coverage missing'
grep -Fq 'examples/typed-multipart/main.vrn' crates/native-build/src/incremental_build/tests/release_native_smoke.rs || fail 'typed multipart generated shard smoke coverage missing'

if ! command -v cargo >/dev/null 2>&1 || ! command -v rustc >/dev/null 2>&1; then
    if [ "${VELRAN_REQUIRE_RUST_TOOLCHAIN:-0}" = "1" ]; then
        fail 'cargo/rustc are required in release mode'
    fi
    echo 'release compile-clean gate: SKIP toolchain execution (static invariants PASS)'
    exit 0
fi

cargo fmt --all -- --check
# Treat warnings in project compilation as release blockers. RUSTFLAGS applies to
# the locked graph too; a dependency warning is also useful release evidence that
# the chosen toolchain/dependency set is not warning-clean.
RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-Dwarnings" cargo check --locked --workspace --all-targets
RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-Dwarnings" cargo test --locked --workspace --no-fail-fast
VELRAN_REQUIRE_RUST_TOOLCHAIN=1 ./tools/check-positive-examples.sh
VELRAN_RELEASE_NATIVE_SMOKE=1 RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-Dwarnings" \
    cargo test --locked -p native-build release_native_positive_smoke_compiles_generated_cdylibs -- --nocapture

echo 'release compile-clean gate: PASS'
