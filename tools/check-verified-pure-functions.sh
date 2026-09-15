#!/bin/sh
set -eu
fail() { echo "FAIL: $*" >&2; exit 1; }

ast=crates/language-core/src/ast_functions.rs
parser=crates/compiler/src/pure_functions.rs
calls=crates/compiler/src/pure_calls.rs
surface=crates/compiler/src/rustlike_surface.rs
native=crates/executable-ir/src/native_scalar.rs
typed=crates/executable-ir/src/typed_numeric.rs
emit=crates/compiler/src/codegen/emit.rs
lower=crates/compiler/src/codegen/typed_lower.rs
planner=crates/executable-ir/src/shard_planner/planning.rs
example=examples/fft4096-x10000/main.vrn

grep -Fq 'pub struct PureFunction' "$ast" || fail 'PureFunction AST missing'
grep -Fq 'must use `i64`, `bool`, `&str`, `&[String]`, `&Struct`, or `&mut [f32; N]`' "$parser" || fail 'pure params are not constrained to verified scalar/borrowed/fixed-array types'
grep -Fq 'cannot mix scalar/borrowed parameters and `&mut [f32; N]`' "$parser" || fail 'mixed pure parameter ownership families are not rejected'
grep -Fq 'MAX_PURE_FIXED_ARRAY_LEN' "$parser" || fail 'pure fixed-array bound missing'
grep -Fq 'parse_pure_compute_statements' "$parser" || fail 'pure body bypasses verified compute parser'
grep -Fq 'validate_pure_recursion_contract' "$parser" || fail 'pure recursion contract validation missing'
grep -Fq 'PureParamType::Int' "$parser" || fail 'i64 value parameters missing from verified pure surface'
grep -Fq 'PureParamType::Bool' "$parser" || fail 'bool value parameters missing from verified pure surface'
grep -Fq 'parse_expr_in_namespace(expression_text' "$calls" || fail 'pure call arguments are not typed expressions'
grep -Fq '__velran_set_call_' crates/compiler/src/control_flow.rs || fail 'assignment from pure helper result is not normalized safely'
grep -Fq 'crate::pure_calls::parse(body, cursor, namespace, known, p)?' crates/compiler/src/control_flow.rs || fail 'handler control-flow cannot parse verified pure function calls'
grep -Fq 'target: None' crates/compiler/src/control_flow.rs || fail 'handler control-flow cannot emit statement-position verified pure calls'
grep -Fq 'cannot borrow `{variable}` mutably more than once' "$calls" || fail 'duplicate mutable alias rejection missing'
grep -Fq 'only Velran callable attributes and #[inline(...)] are allowed' "$surface" || fail 'Rust attribute fail-closed policy missing'
grep -Fq 'rejects_linker_and_abi_attributes' "$surface" || fail 'dangerous Rust attribute regression test missing'
grep -Fq 'lower_pure_function' "$native" || fail 'pure function verified native lowering missing'
grep -Fq 'lower_with_fixed_inputs' "$typed" || fail 'fixed-array typed lowering missing'
grep -Fq 'pure_functions().values()' "$emit" || fail 'pure helper codegen missing'
grep -Fq '#[inline(never)]' "$emit" || fail 'inline never codegen missing'
grep -Fq 'ambient_authority=false' "$emit" || fail 'generated pure authority marker missing'
grep -Fq 'VELRAN_MAX_PURE_CALL_DEPTH' "$emit" || fail 'pure recursion hard depth limit missing'
grep -Fq 'recursion_budget: u16' "$emit" || fail 'scalar pure recursion budget is not threaded through generated helpers'
grep -Fq 'velran_recursion_budget.saturating_sub(1)' crates/compiler/src/codegen/lower.rs || fail 'nested scalar pure calls do not decrement recursion budget'
grep -Fq 'velran_fuel.remaining()' "$lower" || fail 'pure call does not inherit caller fuel budget'
grep -Fq 'velran_state.remaining_alloc()' "$lower" || fail 'pure call does not inherit caller allocation budget'
grep -Fq 'velran_fuel.charge(velran_pure_call.3)' "$lower" || fail 'pure call fuel is not charged back to caller'
grep -Fq 'velran_state.charge_alloc(velran_pure_call.4)' "$lower" || fail 'pure call allocation is not charged back to caller'
grep -Fq 'referenced_pure_functions(program, handlers.values())' "$planner" || fail 'pure functions are not dependency-selected per shard'
! grep -Fq 'program.pure_functions().clone()' "$planner" || fail 'unreferenced pure functions are still cloned into every shard'
grep -Fq '#[inline(never)]' "$example" || fail 'FFT benchmark is not an inline-never function benchmark'
grep -Fq 'fn fft4096(real: &mut [f32; 4096], imag: &mut [f32; 4096])' "$example" || fail 'FFT benchmark helper signature missing'
grep -Fq 'fft4096(&mut real, &mut imag);' "$example" || fail 'FFT benchmark does not call verified helper'
if grep -R -n -E '(^|[^[:alnum:]_])#bench([^[:alnum:]_]|$)' examples crates/compiler/src crates/language-core/src >/dev/null 2>&1; then
  fail 'benchmark-specific #bench language surface introduced'
fi

echo 'PASS: verified pure functions are Rust-like, bounded, alias-safe, budget-accounted, and authority-free'
