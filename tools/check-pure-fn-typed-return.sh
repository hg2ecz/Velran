#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
grep -F 'pub enum PureReturnType' "$ROOT/crates/language-core/src/pure_types.rs" >/dev/null
grep -F 'target: Option<String>' "$ROOT/crates/executable-ir/src/native_scalar.rs" >/dev/null
grep -F 'typed_result_f32' "$ROOT/crates/compiler/src/codegen/emit.rs" >/dev/null
grep -F 'VELRAN_VALUE_F32_INTERNAL' "$ROOT/crates/compiler/src/codegen/emit.rs" >/dev/null
grep -F 'cannot borrow' "$ROOT/crates/compiler/src/pure_calls.rs" >/dev/null
grep -F '#[inline(never)]' "$ROOT/examples/pure-fn-typed-return/main.vrn" >/dev/null
grep -F 'let value = sample_bin(&mut real);' "$ROOT/examples/pure-fn-typed-return/main.vrn" >/dev/null
! grep -R -F 'allow(dead_code)' "$ROOT/crates/compiler/src/codegen" >/dev/null
printf '%s\n' 'pure fn typed return gate: ok'
