#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
S="$ROOT/crates/compiler/src/rust_surface_syntax.rs"
V="$ROOT/crates/language-core/src/values.rs"
grep -q 'let mut ' "$S"
grep -q 'direct_assignment_start' "$S"
grep -q 'split_plain_assignment' "$S"
grep -q '"i64" => Some(Self::Int)' "$V"
grep -q '"f32" => Some(Self::F32)' "$V"
grep -q '"bool" => Some(Self::Bool)' "$V"
grep -q '"Array<f32>" | "Vec<f32>"' "$V"
grep -q '"Vec<String>" => Some(Self::StringList)' "$V"
grep -q '"BTreeMap<String,String>" => Some(Self::StringDict)' "$V"
! grep -q '"Int" | "i64"' "$V"
! grep -q '"Bool" | "bool"' "$V"
! grep -q 'strip_prefix("set ")' "$S"
! grep -R -q 'starts_with("set ") || crate::rust_surface_syntax::direct_assignment_start' \
  "$ROOT/crates/compiler/src/page_statements.rs" \
  "$ROOT/crates/compiler/src/action_statements.rs" \
  "$ROOT/crates/compiler/src/control_flow.rs"
grep -q 'direct_assignment_start(body, cursor)' "$ROOT/crates/compiler/src/page_statements.rs"
grep -q 'direct_assignment_start(body, cursor)' "$ROOT/crates/compiler/src/action_statements.rs"
grep -q 'direct_assignment_start(body, cursor)' "$ROOT/crates/compiler/src/control_flow.rs"
echo 'Velran Rust-like syntax verification: PASS'
