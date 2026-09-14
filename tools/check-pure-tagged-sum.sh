#!/bin/sh
set -eu
fail() { echo "pure-tagged-sum: $*" >&2; exit 1; }
PF=crates/compiler/src/pure_functions.rs
CF=crates/compiler/src/control_flow.rs
NS=crates/executable-ir/src/native_scalar.rs
CG=crates/compiler/src/codegen/emit.rs
LOW=crates/compiler/src/codegen/lower.rs
EX=examples/pure-fn-tagged-sum/main.vrn

grep -Fq 'ComputeStatement::ReturnOption' "$CF" || fail 'Option return syntax lowering missing'
grep -Fq 'ComputeStatement::ReturnResult' "$CF" || fail 'Result return syntax lowering missing'
grep -Fq 'PureReturnType::Option(inner)' "$PF" || fail 'Option return contract missing'
grep -Fq 'PureReturnType::Result { ok, err }' "$PF" || fail 'Result return contract missing'
grep -Fq 'Option/Result return cannot currently cross the mutable numeric helper family' "$PF" || fail 'mutable numeric/sum family must fail closed'
grep -Fq 'pub enum NativePureValueType' "$NS" || fail 'typed native sum payload descriptor missing'
grep -Fq 'ReturnOption {' "$NS" || fail 'typed Option EIR missing'
grep -Fq 'inner_type: NativePureValueType' "$NS" || fail 'typed Option payload descriptor missing'
grep -Fq 'ReturnResult {' "$NS" || fail 'typed Result EIR missing'
grep -Fq 'ok_type: NativePureValueType' "$NS" || fail 'typed Result ok descriptor missing'
grep -Fq 'err_type: NativePureValueType' "$NS" || fail 'typed Result err descriptor missing'
grep -Fq 'PureSum(PureSumValue)' "$CG" || fail 'compiler-owned tagged sum transport missing'
! grep -Fq 'Scalar::Option' "$CG" || fail 'generic Scalar::Option runtime forbidden'
! grep -Fq 'Scalar::Result' "$CG" || fail 'generic Scalar::Result runtime forbidden'
! grep -Fq 'Box<Scalar>' "$CG" || fail 'boxed dynamic Scalar payload forbidden'
grep -Fq 'NativePureValueType::Struct(_)' "$LOW" || fail 'struct sum payload guard missing'
grep -Fq 'sum struct payload lowering is not enabled' "$LOW" || fail 'struct sum payload must remain disabled'
grep -Fq 'v.is_finite()' "$LOW" || fail 'f32 sum payload must remain finite-checked'
grep -Fq 'velran-15-formal-effects' crates/language-core/src/artifact_versions.rs || fail 'language version missing'
grep -Fq 'pub const EXECUTABLE_IR_VERSION: u16 = 25;' crates/executable-ir/src/model.rs || fail 'EIR version missing'
grep -Fq 'rust-aot-50-release-stabilization' crates/compiler/src/codegen/mod.rs || fail 'codegen cache version missing'
test -f "$EX" || fail 'tagged sum example missing'
grep -Fq -- '-> Option<String>' "$EX" || fail 'Option example missing'
grep -Fq -- '-> Result<bool, String>' "$EX" || fail 'Result Ok example missing'
grep -Fq 'return Err(' "$EX" || fail 'Result Err example missing'
grep -qxF 'examples/pure-fn-tagged-sum/main.vrn' tests/manifests/example-entrypoints.txt || fail 'positive manifest missing tagged sum example'
echo 'pure-tagged-sum: ok'
