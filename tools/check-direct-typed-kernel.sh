#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
file=crates/compiler/src/codegen/typed_lower.rs
fail() { echo "direct typed kernel gate: FAIL: $*" >&2; exit 1; }
grep -q 'fn expr_value_source' "$file" || fail 'direct typed expression lowering missing'
if grep -q 'fn expr_source' "$file"; then fail 'legacy Option expression lowering remains'; fi
if grep -q 'typed_f32_array_get(&' "$file"; then fail 'typed kernel still calls array-get helper'; fi
if grep -q 'typed_f32_array_set(&mut' "$file"; then fail 'typed kernel still calls array-set helper'; fi
grep -Fq '[velran_index] = velran_value' "$file" || fail 'direct safe array store missing'
grep -Fq 'if velran_index >=' "$file" || fail 'explicit safe runtime bounds guard missing'
grep -Fq 'IndexProof::Proven' "$file" || fail 'range-proof lowering missing'
grep -Fq '[{} as usize]' "$file" || true
grep -q 'rust-aot-50-release-stabilization' crates/compiler/src/codegen/mod.rs || fail 'codegen cache version not bumped'
echo 'direct typed kernel gate: PASS'
