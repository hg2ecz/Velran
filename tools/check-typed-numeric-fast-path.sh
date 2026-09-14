#!/bin/sh
set -eu
fail() { echo "typed numeric fast-path check: $*" >&2; exit 1; }
grep -q 'pub const CODEGEN_VERSION:' crates/compiler/src/codegen/mod.rs || fail 'codegen version missing'
grep -q 'numeric_fast_path_eligible' crates/executable-ir/src/native_scalar.rs || fail 'typed eligibility missing'
grep -q 'typed_lower::body' crates/compiler/src/codegen/emit.rs || fail 'typed codegen dispatch missing'
grep -q 'Vec<f32>' crates/compiler/src/codegen/typed_lower.rs || fail 'unboxed f32 vector type missing'
grep -q 'TypedExprKind::CollectionIndex' crates/compiler/src/codegen/typed_lower.rs || fail 'typed array read lowering missing'
grep -q 'TypedStatement::ArraySet' crates/compiler/src/codegen/typed_lower.rs || fail 'typed array write lowering missing'
grep -q 'IndexProof::Proven' crates/compiler/src/codegen/typed_lower.rs || fail 'proved index fast path missing'
grep -q 'IndexProof::RuntimeChecked' crates/compiler/src/codegen/typed_lower.rs || fail 'runtime checked index fallback missing'
! grep -q 'Scalar::' crates/compiler/src/codegen/typed_lower.rs || fail 'Scalar enum leaked into typed numeric lowering'
echo 'typed numeric fast-path check: PASS'
