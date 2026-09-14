#!/bin/sh
set -eu
grep -q 'pub const CODEGEN_VERSION' crates/compiler/src/codegen/mod.rs
grep -q 'while {} < {}i64' crates/compiler/src/codegen/typed_lower.rs
grep -q 'IndexProof::Proven' crates/compiler/src/codegen/typed_lower.rs
grep -q 'IndexProof::RuntimeChecked' crates/compiler/src/codegen/typed_lower.rs
grep -q 'usize::try_from(velran_index_value)' crates/compiler/src/codegen/typed_lower.rs
grep -q 'velran_index >= .*\.len()' crates/compiler/src/codegen/typed_lower.rs
grep -q '\[velran_index\] = velran_value' crates/compiler/src/codegen/typed_lower.rs
if grep -q 'get_unchecked' crates/compiler/src/codegen/typed_lower.rs; then
  echo 'unsafe unchecked index lowering found' >&2
  exit 1
fi
if grep -q 'unsafe {' crates/compiler/src/codegen/typed_lower.rs; then
  echo 'unsafe block found in typed lowering' >&2
  exit 1
fi
echo 'safe BCE lowering checks: PASS'
