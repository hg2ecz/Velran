#!/bin/sh
set -eu
fail() { echo "pure-fn-string-list: $*" >&2; exit 1; }
AST=crates/language-core/src/ast.rs
PURE_TYPES=crates/language-core/src/pure_types.rs
PF=crates/compiler/src/pure_functions.rs
PC=crates/compiler/src/pure_calls.rs
CG=crates/compiler/src/codegen/emit.rs
LOWER=crates/compiler/src/codegen/lower.rs
EIR=crates/executable-ir/src/native_scalar.rs
EX=examples/pure-fn-string-list/main.vrn

grep -Fq 'StringList,' "$AST" || grep -Fq 'StringList,' "$PURE_TYPES" || fail 'typed list pure param/return missing'
grep -Fq 'compact == "&[String]"' "$PF" || fail 'Rust-like borrowed list parser missing'
grep -Fq '"Vec<String>" => Ok(PureValueType::StringList)' "$PF" || fail 'owned Vec<String> return parser missing'
grep -Fq 'ValueType::StringList' "$PC" || fail 'call type validation missing'
grep -Fq 'StringList(Arc<Vec<Arc<str>>>)' "$CG" || fail 'shared immutable list representation missing'
! grep -Fq 'StringList(Vec<Arc<str>>)' "$CG" || fail 'unshared generic StringList representation remains'
grep -Fq 'Scalar::StringList(v) => v.clone()' "$LOWER" || fail 'O(1) shared list call transport missing'
grep -Fq 'NativeInputType::StringList' "$EIR" || fail 'verified list call metadata missing'
grep -Fq 'velran-15-formal-effects' crates/language-core/src/artifact_versions.rs || fail 'language version missing'
grep -Fq 'pub const EXECUTABLE_IR_VERSION: u16 = 25;' crates/executable-ir/src/model.rs || fail 'EIR version missing'
grep -Fq 'rust-aot-50-release-stabilization' crates/compiler/src/codegen/mod.rs || fail 'codegen version missing'
grep -Fq 'fn echo_tags(tags: &[String]) -> Vec<String>' "$EX" || fail 'example function missing'
grep -Fq 'let echoed = echo_tags(&tags);' "$EX" || fail 'example call missing'
grep -qxF 'examples/pure-fn-string-list/main.vrn' tests/manifests/example-entrypoints.txt || fail 'positive manifest missing example'

echo 'pure-fn-string-list: ok'
