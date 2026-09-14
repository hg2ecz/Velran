#!/bin/sh
set -eu
fail() { echo "pure-fn-borrowed-string: $*" >&2; exit 1; }

AST=crates/language-core/src/ast_functions.rs
PARSER=crates/compiler/src/pure_functions.rs
CALLS=crates/compiler/src/pure_calls.rs
EIR=crates/executable-ir/src/native_scalar.rs
MODEL=crates/executable-ir/src/model.rs
EMIT=crates/compiler/src/codegen/emit.rs
LOWER=crates/compiler/src/codegen/lower.rs
EX=examples/pure-fn-borrowed-string/main.vrn

for f in "$AST" "$PARSER" "$CALLS" "$EIR" "$MODEL" "$EMIT" "$LOWER" "$EX"; do [ -f "$f" ] || fail "missing $f"; done

grep -Fq 'pub enum PureParamType' "$AST" || fail 'typed pure parameter enum missing'
grep -Fq 'F32ArrayMut(u32)' "$AST" || fail 'bounded mutable f32 array parameter missing'
grep -Fq 'Str,' "$AST" || fail 'borrowed string parameter missing'
grep -Fq 'compact == "&str"' "$PARSER" || fail '&str parser support missing'
grep -Fq 'cannot mix string/list borrows and `&mut [f32; N]`' "$PARSER" || fail 'mixed string/list vs mutable-array ownership family must fail closed'
grep -Fq 'has type `&str` and must be passed as `&variable`' "$CALLS" || fail 'Rust-like borrowed call syntax not enforced'
grep -Fq 'PureParamType::Str => NativeInputType::String' "$EIR" || fail 'borrowed string EIR lowering missing'
grep -Fq 'param_types: Vec<NativeInputType>' "$EIR" || fail 'pure call parameter contract missing from EIR'
grep -Fq 'velran_arg_{index}: std::sync::Arc<str>' "$EMIT" || fail 'owned safe generated representation missing'
grep -Fq 'Scalar::String(velran_arg_{index})' "$EMIT" || fail 'safe string helper binder missing'
grep -Fq 'Scalar::String(v) => v.clone()' "$LOWER" || fail 'caller must share immutable string ownership safely'
grep -Fq 'velran_fuel.remaining()' "$LOWER" || fail 'pure helper must inherit remaining fuel budget'
grep -Fq 'velran_state.remaining_alloc()' "$LOWER" || fail 'pure helper must inherit remaining allocation budget'
grep -Fq 'velran_fuel.charge(velran_pure_call.fuel_used)' "$LOWER" || fail 'helper fuel must be charged back to caller'
grep -Fq 'velran_state.charge_alloc(velran_pure_call.allocated)' "$LOWER" || fail 'helper allocations must be charged back to caller'
grep -Fq 'Scalar::F32(v) if v.is_finite()' "$EMIT" || fail 'generic pure f32 return must fail closed on non-finite values'
grep -Fq 'pub const EXECUTABLE_IR_VERSION: u16 = 25;' "$MODEL" || fail 'EIR version not bumped'
grep -Fq 'rust-aot-50-release-stabilization' crates/compiler/src/codegen/mod.rs || fail 'codegen cache version not bumped'
grep -Fq 'velran-15-formal-effects' crates/language-core/src/artifact_versions.rs || fail 'language version not bumped'
grep -Fq 'fn normalized_len(input: &str) -> i64' "$EX" || fail 'example borrowed helper missing'
grep -Fq 'normalized_len(&text)' "$EX" || fail 'example Rust-like borrowed call missing'
grep -qxF 'examples/pure-fn-borrowed-string/main.vrn' tests/manifests/example-entrypoints.txt || fail 'positive manifest missing example'

# Do not weaken the generated shard safety posture.
grep -Fq '#![forbid(unsafe_code)]' "$EMIT" || fail 'generated unsafe prohibition missing'
grep -Fq '#![deny(warnings)]' "$EMIT" || fail 'generated warning denial missing'

echo 'pure borrowed-string function verification passed'
