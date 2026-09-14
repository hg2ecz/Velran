#!/bin/sh
set -eu
fail() { echo "native request input check: $*" >&2; exit 1; }

grep -q 'pub const RUNTIME_ABI_VERSION: u32 = 10;' crates/runtime-abi/src/lib.rs || fail 'ABI v10 typed input contract missing'
grep -q 'pub enum RequestValue' crates/runtime-abi/src/input.rs || fail 'safe host request value API missing'
grep -q 'pub struct VelranInputValue' crates/runtime-abi/src/input.rs || fail 'fixed wire input layout missing'
grep -q 'pub const MAX_INPUT_FIELDS: usize = 64;' crates/runtime-abi/src/input.rs || fail 'request field cap missing'
grep -q 'MAX_NATIVE_INPUT_STRING_BYTES: usize = 1_048_576' crates/executable-ir/src/native_input.rs || fail 'verified native string input bound missing'
grep -q 'ValidationKind::Length' crates/executable-ir/src/native_input.rs || fail 'route string length contract missing'
grep -q 'NativeInputType::String' crates/compiler/src/codegen/emit.rs || fail 'typed generated input binding missing'
grep -q 'from_utf8' crates/native-build/src/rustc_backend/abi_input_validation.rs || fail 'ABI shim UTF-8 validation missing'
grep -q 'VELRAN_MAX_INPUT_FIELDS' crates/native-build/src/rustc_backend/abi_shim.rs || fail 'ABI shim field cap missing'
grep -q 'raw.aux == 0' crates/native-build/src/rustc_backend/abi_shim.rs || fail 'ABI aux-field primitive validation missing'
! grep -R -n 'request_handle' crates/runtime-abi crates/native-runtime crates/compiler crates/native-build >/dev/null || fail 'opaque request_handle survived typed input migration'
! grep -R -n 'unsafe ' crates/compiler/src/codegen crates/executable-ir/src/native_input.rs >/dev/null || fail 'unsafe escaped into generated/verifier request boundary'

printf '%s\n' 'native typed request input verification passed'
