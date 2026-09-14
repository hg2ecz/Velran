#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
file="$root/crates/compiler/src/codegen/emit.rs"
grep -Fq 'if value.is_ascii() { value.len() } else { value.chars().count() }' "$file"
grep -Fq 'if trimmed.len() == value.len() { return Some(Scalar::String(value)); }' "$file"
grep -Fq 'if from == to { return Some(Scalar::String(text)); }' "$file"
grep -Fq 'if count == 0 { return Some(Scalar::String(text)); }' "$file"
grep -Fq 'if text.is_ascii() {' "$file"
grep -Fq 'if count == 1 { return Some(Scalar::String(text)); }' "$file"
grep -Fq 'is_some_and(|existing| existing.as_ref() == value.as_ref())' "$file"
grep -Fq 'if !items.contains_key(key.as_ref()) { return Some(Scalar::StringDict(items)); }' "$file"
echo 'string/map fastpath verification passed'
