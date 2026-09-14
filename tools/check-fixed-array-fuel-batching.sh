#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
grep -q 'rust-aot-50-release-stabilization' crates/compiler/src/codegen/mod.rs
grep -q 'NumericCollectionKind::FixedArray(size)' crates/compiler/src/codegen/typed_lower.rs
grep -q 'velran_state.charge_alloc' crates/compiler/src/codegen/typed_lower.rs
grep -q 'fn is_batchable' crates/compiler/src/codegen/typed_lower.rs
grep -q 'fuel = fuel.saturating_add' crates/compiler/src/codegen/typed_lower.rs
grep -q '1..=16_384' crates/executable-ir/src/typed_numeric.rs
echo 'fixed-array + fuel batching gate: PASS'
