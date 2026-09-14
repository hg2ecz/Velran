#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
grep -q 'pub const RUNTIME_ABI_VERSION: u32 = 10;' "$root/crates/runtime-abi/src/lib.rs"
grep -q 'HOST_OP_OUTBOUND_GET_STATUS' "$root/crates/runtime-abi/src/host.rs"
grep -q 'dispatch_with_host' "$root/crates/native-runtime/src/application_runtime/engine.rs"
grep -q 'NativeHostBridge' "$root/crates/server/src/native_host.rs"
grep -q 'HostOutboundStatus' "$root/crates/executable-ir/src/native_scalar.rs"
grep -q 'velran_host.outbound_status' "$root/crates/compiler/src/codegen/lower.rs"
grep -q 'host: \*const VelranHostApi' "$root/crates/native-build/src/rustc_backend/abi_shim.rs"
if grep -n 'args.extend.*--extern\|\"--extern\".into' "$root/crates/native-build/src/rustc_backend/build.rs"; then
    echo 'generated shard build gained an external crate dependency' >&2
    exit 1
fi
printf 'native host ABI: PASS\n'
