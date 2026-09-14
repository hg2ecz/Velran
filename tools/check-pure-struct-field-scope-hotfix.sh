#!/usr/bin/env bash
set -euo pipefail
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
FILE="$ROOT/crates/compiler/src/codegen/emit.rs"
grep -Fq '(Scalar::Struct(v),field)=>super::pure_struct_field_value(v,field,state)' "$FILE"
grep -Fq 'pub(crate) fn pure_struct_field_value' "$FILE"
if grep -Fq '(Scalar::Struct(v),field)=>pure_struct_field_value(v,field,state)' "$FILE"; then
  echo 'unqualified pure_struct_field_value call remains in velran_support' >&2
  exit 1
fi
echo 'pure struct field scope hotfix: ok'
