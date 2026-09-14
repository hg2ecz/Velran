#!/bin/sh
set -eu
fail() { echo "pure-composite-type-foundation: $*" >&2; exit 1; }
PT=crates/language-core/src/pure_types.rs
PF=crates/compiler/src/pure_functions.rs
CG=crates/compiler/src/codegen/emit.rs

grep -Fq 'pub enum PureValueType' "$PT" || fail 'PureValueType missing'
grep -Fq 'Struct(String)' "$PT" || fail 'nominal struct value type missing'
grep -Fq 'Option(PureValueType)' "$PT" || fail 'typed Option return missing'
grep -Fq 'Result {' "$PT" || fail 'typed Result return missing'
grep -Fq 'ok: PureValueType' "$PT" || fail 'typed Result ok payload missing'
grep -Fq 'err: PureValueType' "$PT" || fail 'typed Result err payload missing'
grep -Fq 'unwrap_generic(raw, "Option")' "$PF" || fail 'Rust Option<T> return syntax parser missing'
grep -Fq 'unwrap_generic(raw, "Result")' "$PF" || fail 'Rust Result<T,E> return syntax parser missing'
grep -Fq 'program.json_schema(&symbol).is_some()' "$PF" || fail 'nominal struct return resolution missing'
grep -Fq 'return StructName {{ ... }}' "$PF" || fail 'typed struct return contract missing'
! grep -Fq 'Scalar::Object' "$CG" || fail 'dynamic object runtime must not be introduced'
! grep -Fq 'Scalar::Option' "$CG" || fail 'generic Option runtime must not be introduced'
! grep -Fq 'Scalar::Result' "$CG" || fail 'generic Result runtime must not be introduced'
grep -Fq 'velran-15-formal-effects' crates/language-core/src/artifact_versions.rs || fail 'language version missing'
grep -Fq 'pub const EXECUTABLE_IR_VERSION: u16 = 25;' crates/executable-ir/src/model.rs || fail 'EIR version missing'
grep -Fq 'rust-aot-50-release-stabilization' crates/compiler/src/codegen/mod.rs || fail 'codegen version missing'
echo 'pure-composite-type-foundation: ok'
