#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
IR="$ROOT/crates/executable-ir/src/typed_numeric.rs"
CODEGEN="$ROOT/crates/compiler/src/codegen/typed_lower.rs"
EMIT="$ROOT/crates/compiler/src/codegen/emit.rs"

grep -q 'pub struct LocalId(pub u32)' "$IR"
grep -q 'pub struct BlockId(pub u32)' "$IR"
grep -q 'pub struct VerifiedNumericBody' "$IR"
grep -q 'pub fn cfg(&self)' "$IR"
grep -q 'Local(LocalId)' "$IR"
grep -q 'CollectionIndex { collection: LocalId' "$IR"
grep -q 'CfgNodeKind::LoopCondition' "$IR"
grep -q 'format!("velran_l{}", id.0)' "$CODEGEN"
grep -q 'body.numeric_body()' "$EMIT"
grep -q 'typed_lower::body(numeric_body)' "$EMIT"

if grep -q 'ScalarExpr::Variable' "$CODEGEN"; then
  echo 'typed codegen still depends on name-based ScalarExpr variables' >&2
  exit 1
fi
if grep -q 'local_ident(name)' "$CODEGEN"; then
  echo 'typed codegen still emits name-based locals' >&2
  exit 1
fi

echo 'localid typed cfg checks passed'
