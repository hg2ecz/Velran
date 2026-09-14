#!/bin/sh
set -eu
fail() { echo "inherent impl support check: FAIL: $*" >&2; exit 1; }

# Language-core metadata and compile pipeline must both exist.
grep -Fq 'pub struct InherentMethod' crates/language-core/src/ast_functions.rs || fail 'InherentMethod metadata missing'
grep -Fq 'register_inherent_impls' crates/compiler/src/compile_pipeline.rs || fail 'impl registration missing'
grep -Fq 'lower_inherent_impls' crates/compiler/src/compile_pipeline.rs || fail 'impl lowering missing'

# Method bodies must reuse verified pure lowering, never raw Rust passthrough.
grep -Fq 'parse_pure_functions(&synthetic' crates/compiler/src/inherent_impl.rs || fail 'method body does not reuse pure verifier'
grep -Fq 'self_value: &' crates/compiler/src/inherent_impl.rs || fail '&self receiver lowering missing'
grep -Fq 'PureParamType::Struct' crates/compiler/src/pure_functions.rs || fail 'borrowed struct pure parameter missing'
grep -Fq 'NativeInputType::Struct' crates/executable-ir/src/native_scalar.rs || fail 'struct native transport missing'
grep -Fq 'PureStructValue::S{id}' crates/compiler/src/codegen/lower.rs || fail 'struct method-call transport missing'
grep -Fq 'replace_identifier(method.return_suffix, "Self", block.target)' crates/compiler/src/inherent_impl.rs || fail 'Self return-type lowering missing'
grep -Fq 'validate_public_pure_api' crates/compiler/src/inherent_impl.rs || fail 'public method API type visibility validation missing'

# Unsafe receiver/trait surfaces stay fail-closed in this iteration.
grep -Fq 'immutable `&self` receiver' crates/compiler/src/inherent_impl.rs || fail '&mut/owned self rejection missing'
grep -Fq 'trait impls are not supported yet' crates/compiler/src/inherent_impl.rs || fail 'trait impl rejection missing'

# Regression tests must cover success and security failures.
grep -Fq 'inherent_self_method_compiles_through_verified_pure_path' crates/compiler/src/tests/inherent_impl_compile_tests.rs || fail 'positive impl regression test missing'
grep -Fq 'unsafe_inside_impl_is_rejected_before_lowering' crates/compiler/src/tests/inherent_impl_compile_tests.rs || fail 'unsafe impl regression test missing'
grep -Fq 'mutable_receiver_is_rejected_fail_closed' crates/compiler/src/tests/inherent_impl_compile_tests.rs || fail 'mutable receiver regression test missing'
grep -Fq 'associated_constructor_with_self_compiles_through_verified_pure_path' crates/compiler/src/tests/inherent_impl_compile_tests.rs || fail 'associated Self constructor regression test missing'
grep -Fq 'public_method_cannot_expose_private_struct_in_return_type' crates/compiler/src/tests/inherent_impl_compile_tests.rs || fail 'public method type-leak regression test missing'
grep -Fq 'cross_package_struct_constructor_and_method_compile' crates/compiler/src/tests/package_compile_tests.rs || fail 'cross-package object API regression test missing'
test -f examples/inherent-methods/main.vrn || fail 'canonical inherent-method example missing'
grep -Fq 'check examples/inherent-methods/main.vrn' verify.sh || fail 'inherent-method example is not compiler-checked by verify.sh'

echo 'inherent impl verified-lowering support: PASS'
