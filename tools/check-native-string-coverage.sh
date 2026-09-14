#!/bin/sh
set -eu
fail() { echo "native string coverage check: $*" >&2; exit 1; }

grep -q 'pub const RUNTIME_ABI_VERSION: u32 = 10;' crates/runtime-abi/src/lib.rs || fail 'ABI v10 allocation contract missing'
grep -q 'pub allocated_bytes: u64' crates/runtime-abi/src/result.rs || fail 'allocation accounting result missing'
grep -q 'ScalarType::StringList' crates/executable-ir/src/native_scalar.rs || fail 'bounded StringList verified type missing'
grep -q 'BuiltinFunction::SplitBounded => ScalarBuiltin::SplitBounded' crates/executable-ir/src/native_scalar.rs || fail 'splitBounded native lowering missing'
! grep -q 'BuiltinFunction::Split => ScalarBuiltin' crates/executable-ir/src/native_scalar.rs || fail 'unbounded split must remain unsupported by native lowering'
grep -q 'charge_alloc' crates/compiler/src/codegen/emit.rs || fail 'native allocation charging missing'
grep -q 'STATUS_MEMORY_EXCEEDED' crates/compiler/src/codegen/emit.rs || fail 'memory-limit status missing'
grep -q 'piece_count > max_items' crates/compiler/src/codegen/emit.rs || fail 'bounded split runtime cap missing'
grep -q 'if !state.charge_alloc(cost)' crates/compiler/src/codegen/emit.rs || fail 'bounded split allocation budget missing'
! grep -R -n 'std::fs::\|std::net::\|std::process::Command' crates/compiler/src/codegen >/dev/null || fail 'generated code gained ambient authority'

printf '%s\n' 'native string/bounded-collection coverage verification passed'
