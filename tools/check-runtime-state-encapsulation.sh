#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
fail(){ echo "FAIL: $*" >&2; exit 1; }
EMIT=crates/compiler/src/codegen/emit.rs
LOWER=crates/compiler/src/codegen/typed_lower.rs
grep -Fq 'pub(crate) fn allocated(&self) -> u64 { self.allocated }' "$EMIT" || fail 'RuntimeState allocated accessor missing'
grep -Fq 'velran_state.allocated()' "$LOWER" || fail 'pure-call error path does not use RuntimeState accessor'
if grep -R --line-number --include='*.rs' 'velran_state\.allocated[^()]' crates/compiler/src/codegen | grep -v 'emit.rs'; then
  fail 'direct RuntimeState allocated field access escaped support module'
fi
grep -Fq 'rust-aot-50-release-stabilization' crates/compiler/src/codegen/mod.rs || fail 'codegen cache version not bumped'
echo 'PASS: RuntimeState encapsulation preserved in generated code'
