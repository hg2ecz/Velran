#!/bin/sh
set -eu
fail(){ echo "pure sum methods gate failed: $*" >&2; exit 1; }
EX=examples/pure-fn-tagged-sum/main.vrn
grep -Fq '.is_some()' "$EX" || fail 'missing is_some example'
grep -Fq '.is_ok()' "$EX" || fail 'missing is_ok example'
grep -Fq '.is_err()' "$EX" || fail 'missing is_err example'
grep -Fq '.unwrap_or(' "$EX" || fail 'missing unwrap_or example'
grep -Fq 'pub use crate::sum_types::PureSumPredicate;' crates/language-core/src/ast.rs || fail 'typed predicate AST re-export missing'
grep -Fq 'pub enum PureSumPredicate {' crates/language-core/src/sum_types.rs || fail 'typed predicate enum missing'
grep -Fq 'IsSome' crates/language-core/src/sum_types.rs || fail 'Option predicate variants missing'
grep -Fq 'IsErr' crates/language-core/src/sum_types.rs || fail 'Result predicate variants missing'
grep -Fq 'PureSumUnwrapOr' crates/language-core/src/ast.rs || fail 'typed unwrap_or AST missing'
grep -Fq 'SumPredicate' crates/executable-ir/src/native_scalar.rs || fail 'verified predicate IR missing'
grep -Fq 'SumUnwrapOr' crates/executable-ir/src/native_scalar.rs || fail 'verified unwrap IR missing'
! grep -R -Fq 'Scalar::Option' crates || fail 'generic Scalar::Option forbidden'
! grep -R -Fq 'Scalar::Result' crates || fail 'generic Scalar::Result forbidden'
! grep -R -Fq 'Box<Scalar>' crates || fail 'boxed dynamic sum payload forbidden'
grep -Fq 'velran-15-formal-effects' crates/language-core/src/artifact_versions.rs || fail 'language version missing'
grep -Fq 'pub const EXECUTABLE_IR_VERSION: u16 = 25;' crates/executable-ir/src/model.rs || fail 'EIR version missing'
grep -Fq 'rust-aot-50-release-stabilization' crates/compiler/src/codegen/mod.rs || fail 'codegen version missing'
echo 'pure sum methods gate: PASS'
