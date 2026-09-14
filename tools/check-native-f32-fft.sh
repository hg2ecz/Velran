#!/bin/sh
set -eu
fail() { echo "native F32/FFT verification failed: $*" >&2; exit 1; }
grep -q 'F32ArraySet' crates/executable-ir/src/native_scalar.rs || fail 'F32 array mutation missing from EIR'
grep -q 'BuiltinFunction::Sin => ScalarBuiltin::Sin' crates/executable-ir/src/native_scalar.rs || fail 'sin lowering missing'
grep -q 'BuiltinFunction::MonotonicNanos => ScalarBuiltin::MonotonicNanos' crates/executable-ir/src/native_scalar.rs || fail 'monotonic timer lowering missing'
grep -q 'f32_array_set(&mut' crates/compiler/src/codegen/lower.rs || fail 'in-place F32 array codegen missing'
grep -q 'MAX_F32_ARRAY_ELEMENTS: usize = 1_048_576' crates/compiler/src/codegen/numeric_support.rs || fail 'hard F32 array bound missing'
grep -q 'try_reserve_exact' crates/compiler/src/codegen/numeric_support.rs || fail 'fallible F32 array allocation missing'
grep -q 'value.is_finite()' crates/compiler/src/codegen/numeric_support.rs || fail 'finite F32 enforcement missing'
grep -q 'pub const CODEGEN_VERSION:' crates/compiler/src/codegen/mod.rs || fail 'cache-busting codegen version missing'
grep -Eq 'EXECUTABLE_IR_VERSION: u16 = ([9]|[1-9][0-9]+)' crates/executable-ir/src/model.rs || fail 'EIR version too old for typed range-aware FFT path'
echo 'native F32/FFT verification passed'
