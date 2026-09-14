#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
fail(){ echo "f32-hoisting: $*" >&2; exit 1; }
file=crates/compiler/src/codegen/typed_lower.rs
grep -Fq 'rust-aot-50-release-stabilization' crates/compiler/src/codegen/mod.rs || fail 'codegen version missing'
grep -Fq 'fn raw_f32_value_source' "$file" || fail 'raw f32 expression lowering missing'
grep -Fq 'return finite_f32_value(raw_f32_value_source(body, expr));' "$file" || fail 'f32 boundary validation missing'
grep -Fq 'ScalarBuiltin::Sin => format!("({}).sin()", a(0))' "$file" || fail 'sin still performs nested finite wrapper'
grep -Fq 'ScalarBuiltin::Cos => format!("({}).cos()", a(0))' "$file" || fail 'cos still performs nested finite wrapper'
printf '%s\n' 'f32 expression-check hoisting verification passed'
