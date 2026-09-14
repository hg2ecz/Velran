#!/bin/sh
set -eu
fail() { echo "pure-fn-owned-string: $*" >&2; exit 1; }
AST=crates/language-core/src/ast.rs
PURE_TYPES=crates/language-core/src/pure_types.rs
PARSER=crates/compiler/src/pure_functions.rs
CALLS=crates/compiler/src/pure_calls.rs
EIR=crates/executable-ir/src/native_scalar.rs
EMIT=crates/compiler/src/codegen/emit.rs
LOWER=crates/compiler/src/codegen/lower.rs
EX=examples/pure-fn-owned-string/main.vrn
for f in "$AST" "$PURE_TYPES" "$PARSER" "$CALLS" "$EIR" "$EMIT" "$LOWER" "$EX"; do [ -f "$f" ] || fail "missing $f"; done
grep -Fq 'String,' "$PURE_TYPES" || fail 'PureValueType::String missing'
grep -Fq '"String" => Ok(PureValueType::String)' "$PARSER" || fail 'String return parser missing'
grep -Fq 'PureValueType::String => ValueType::String' "$PARSER" || fail 'String return contract typecheck missing'
grep -Fq 'PureReturnType::Value(PureValueType::String) => StaticType::trusted_scalar(ValueType::String)' "$CALLS" || fail 'String call type missing'
grep -Fq 'PureReturnType::Value(PureValueType::String)' "$EIR" || fail 'String pure return EIR lowering missing'
grep -Fq 'pub(crate) struct PureScalarResult' "$EMIT" || fail 'safe internal pure result missing'
grep -Fq 'value: Option<Scalar>' "$EMIT" || fail 'owned scalar result value missing'
grep -Fq 'super::lower::pure_body(function.body())' "$EMIT" || fail 'pure helper lowering mode missing'
grep -Fq 'PureScalarResult {' "$EMIT" || fail 'pure helper result construction missing'
grep -Fq 'NativeScalarType::String => "Scalar::String(_) => true"' "$LOWER" || fail 'String caller verification missing'
grep -Fq 'velran_pure_call.fuel_used' "$LOWER" || fail 'helper fuel chargeback missing'
grep -Fq 'velran_pure_call.allocated' "$LOWER" || fail 'helper allocation chargeback missing'
grep -Fq '#![forbid(unsafe_code)]' "$EMIT" || fail 'unsafe prohibition missing'
grep -Fq 'velran-15-formal-effects' crates/language-core/src/artifact_versions.rs || fail 'language version missing'
grep -Fq 'pub const EXECUTABLE_IR_VERSION: u16 = 25;' crates/executable-ir/src/model.rs || fail 'EIR version missing'
grep -Fq 'rust-aot-50-release-stabilization' crates/compiler/src/codegen/mod.rs || fail 'codegen cache version missing'
grep -Fq 'fn normalize(input: &str) -> String' "$EX" || fail 'owned String example missing'
grep -Fq 'normalize(&text)' "$EX" || fail 'owned String call example missing'
grep -qxF 'examples/pure-fn-owned-string/main.vrn' tests/manifests/example-entrypoints.txt || fail 'positive manifest missing example'
# Explicitly reject pointer/unsafe shortcuts in the new path.
! grep -Eq 'PureScalarResult.*\*const|PureScalarResult.*\*mut|transmute|from_raw' "$EMIT" || fail 'raw pointer ownership shortcut detected'
echo 'pure owned-string function verification passed'
