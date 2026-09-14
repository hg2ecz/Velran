#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"
grep -Fq 'rust-aot-50-release-stabilization' crates/compiler/src/codegen/mod.rs
grep -Fq 'fn counted_straight_line_loop' crates/compiler/src/codegen/typed_lower.rs
grep -Fq 'velran_loop_remaining' crates/compiler/src/codegen/typed_lower.rs
grep -Fq 'velran_loop_fuel' crates/compiler/src/codegen/typed_lower.rs
grep -Fq 'i128::from' crates/compiler/src/codegen/typed_lower.rs
printf '%s\n' 'PASS: counted loop fuel hoisting'
