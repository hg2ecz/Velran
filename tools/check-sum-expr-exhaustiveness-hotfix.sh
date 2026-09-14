#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
N="$ROOT/crates/executable-ir/src/native_scalar.rs"
T="$ROOT/crates/executable-ir/src/typed_numeric.rs"
grep -q 'ScalarExpr::SumPredicate { .. } => NativeScalarType::Bool' "$N"
grep -q 'ScalarExpr::SumUnwrapOr { base_type, .. } => match base_type' "$N"
PRED_COUNT=$(grep -c 'ScalarExpr::SumPredicate { .. }' "$T")
UNWRAP_COUNT=$(grep -c 'ScalarExpr::SumUnwrapOr { .. } => return None' "$T")
[ "$PRED_COUNT" -ge 2 ]
[ "$UNWRAP_COUNT" -ge 2 ]
! grep -q 'let (value_type, is_result)' "$N"
echo 'sum-expression exhaustiveness hotfix: PASS'
