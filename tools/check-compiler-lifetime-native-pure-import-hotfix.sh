#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOWER="$ROOT/crates/compiler/src/codegen/lower.rs"
TYPED="$ROOT/crates/compiler/src/codegen/typed_scalar_lower.rs"

grep -Fq "fn helper<'a>(mode: LowerMode, handler: &'a str, pure: &'a str) -> &'a str" "$LOWER"
grep -Fq "NativePureValueType" "$TYPED"
grep -Fq 'use executable_ir::{' "$TYPED"
grep -Fq 'NativePureValueType' "$TYPED"

echo "compiler lifetime/native-pure import hotfix: PASS"
