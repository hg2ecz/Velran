#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
grep -q 'pub enum NumericElementType' crates/executable-ir/src/typed_numeric.rs
grep -q 'pub enum NumericCollectionKind' crates/executable-ir/src/typed_numeric.rs
grep -q 'FixedArray(u32)' crates/executable-ir/src/typed_numeric.rs
grep -q 'SharedSlice' crates/executable-ir/src/typed_numeric.rs
grep -q 'MutableSlice' crates/executable-ir/src/typed_numeric.rs
grep -q 'pub enum NumericType' crates/executable-ir/src/typed_numeric.rs
grep -q 'Collection(NumericCollectionType)' crates/executable-ir/src/typed_numeric.rs
grep -q 'ArrayNew { element: NumericElementType' crates/executable-ir/src/typed_numeric.rs
grep -q 'ArraySet { array: LocalId' crates/executable-ir/src/typed_numeric.rs
if grep -q 'TypedExprKind::F32ArrayNew' crates/executable-ir/src/typed_numeric.rs; then echo 'typed IR still contains F32ArrayNew' >&2; exit 1; fi
if grep -q 'TypedStatement::F32ArraySet' crates/executable-ir/src/typed_numeric.rs; then echo 'typed IR still contains F32ArraySet' >&2; exit 1; fi
grep -q 'rust-aot-50-release-stabilization' crates/compiler/src/codegen/mod.rs
grep -q 'pub const EXECUTABLE_IR_VERSION: u16 = 25;' crates/executable-ir/src/model.rs
echo 'generic typed collection checks: PASS'
