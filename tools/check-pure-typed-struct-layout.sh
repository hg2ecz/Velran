#!/bin/sh
set -eu
fail() { echo "pure-typed-struct-layout: $*" >&2; exit 1; }
PF=crates/compiler/src/pure_functions.rs
PS=crates/compiler/src/pure_struct_return.rs
EX=crates/executable-ir/src/native_scalar.rs
CG=crates/compiler/src/codegen/emit.rs
LOW=crates/compiler/src/codegen/lower.rs

grep -Fq 'ComputeStatement::ReturnStruct' "$EX" || fail 'typed struct statement lowering missing'
grep -Fq 'PureStruct(String)' crates/compiler/src/handler_types.rs || fail 'nominal handler struct type missing'
grep -Fq 'pure struct literal is unclosed' "$PS" || fail 'bounded struct literal parser missing'
grep -Fq 'must initialize exactly' "$PS" || fail 'exact field-set validation missing'
grep -Fq 'duplicate pure struct field' "$PS" || fail 'duplicate field rejection missing'
grep -Fq 'Scalar::Struct(PureStructValue::S' "$LOW" || fail 'nominal generated struct construction missing'
grep -Fq 'pub(crate) enum PureStructValue' "$CG" || fail 'typed generated struct enum missing'
grep -Fq 'pure_struct_field_value' "$CG" || fail 'typed field accessor missing'
! grep -R -Fq 'HashMap<String, Scalar>' crates/compiler/src/codegen || fail 'dynamic object map transport forbidden'
! grep -R -Fq 'Scalar::Object' crates/compiler/src/codegen || fail 'dynamic object scalar forbidden'
grep -Fq 'struct return cannot currently cross the mutable numeric helper family' "$PF" || fail 'numeric/owned family isolation missing'
grep -Fq 'velran-15-formal-effects' crates/language-core/src/artifact_versions.rs || fail 'language version missing'
grep -Fq 'pub const EXECUTABLE_IR_VERSION: u16 = 25;' crates/executable-ir/src/model.rs || fail 'EIR version missing'
grep -Fq 'rust-aot-50-release-stabilization' crates/compiler/src/codegen/mod.rs || fail 'codegen version missing'

EX=examples/pure-fn-typed-struct/main.vrn
[ -f "$EX" ] || fail 'typed struct example missing'
grep -qF 'fn summarize(input: &str) -> Summary' "$EX" || fail 'typed struct return example missing'
grep -qF 'return Summary {' "$EX" || fail 'typed struct literal example missing'
grep -qF 'summary.normalized' "$EX" || fail 'typed struct field access example missing'
grep -qxF 'examples/pure-fn-typed-struct/main.vrn' tests/manifests/example-entrypoints.txt || fail 'positive manifest missing typed struct example'

echo 'pure-typed-struct-layout: ok'
