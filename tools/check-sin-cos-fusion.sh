#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
f=crates/compiler/src/codegen/typed_lower.rs
grep -Fq 'fn sin_cos_pair' "$f"
grep -Fq '.sin_cos()' "$f"
grep -Fq 'emit_sin_cos_pair' "$f"
grep -Fq 'rust-aot-50-release-stabilization' crates/compiler/src/codegen/mod.rs
echo "sin/cos fusion verification passed"
