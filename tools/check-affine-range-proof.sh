#!/usr/bin/env bash
set -euo pipefail
f=crates/executable-ir/src/typed_numeric.rs
c=crates/compiler/src/codegen/typed_lower.rs
fail(){ echo "FAIL: $*" >&2; exit 1; }
grep -Fq 'fn prove_static_index(' "$f" || fail 'static index proof missing'
grep -Fq 'fn affine_induction_offset(' "$f" || fail 'affine index recognizer missing'
grep -Fq 'checked_add(offset)' "$f" || fail 'affine lower/upper arithmetic must be checked'
grep -Fq 'return IndexProof::RuntimeChecked' "$f" || fail 'fail-closed runtime fallback missing'
grep -Fq 'expr_value_source(body, index)' "$c" || fail 'proven index codegen must preserve original index expression'
grep -Fq 'affine_index_stays_runtime_checked_when_shift_escapes_array' "$f" || fail 'unsafe affine regression test missing'
grep -Fq 'rust-aot-50-release-stabilization' crates/compiler/src/codegen/mod.rs || fail 'codegen cache version not bumped'
echo 'PASS: affine/static range proofs are checked, fail-closed, and cache-versioned'
