#!/usr/bin/env sh
set -eu
fail() { echo "check-range-proof-metadata: FAIL: $*" >&2; exit 1; }

grep -q 'pub struct RangeProofId' crates/executable-ir/src/typed_numeric.rs || fail 'RangeProofId missing'
grep -q 'pub enum IndexProof' crates/executable-ir/src/typed_numeric.rs || fail 'IndexProof missing'
grep -q 'fn recognize_loop' crates/executable-ir/src/typed_numeric.rs || fail 'loop recognition missing'
grep -q 'fn prove_index' crates/executable-ir/src/typed_numeric.rs || fail 'index proof construction missing'
grep -q 'range_proofs: Vec<RangeProof>' crates/executable-ir/src/typed_numeric.rs || fail 'proof table missing'
grep -Eq 'EXECUTABLE_IR_VERSION: u16 = ([9]|[1-9][0-9]+)' crates/executable-ir/src/model.rs || fail 'EIR version is older than range-proof metadata'
grep -q 'IndexProof::Proven' crates/compiler/src/codegen/typed_lower.rs || fail 'codegen does not consume index proofs'
grep -q 'IndexProof::RuntimeChecked' crates/compiler/src/codegen/typed_lower.rs || fail 'runtime bounds-check fallback missing'
grep -q 'if velran_index >=' crates/compiler/src/codegen/typed_lower.rs || fail 'runtime bounds check is not emitted fail-closed'
echo 'check-range-proof-metadata: PASS'
