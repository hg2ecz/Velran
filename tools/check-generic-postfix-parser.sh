#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
primary="$root/crates/compiler/src/expression_primary.rs"
postfix="$root/crates/compiler/src/expression_postfix.rs"
registry="$root/crates/compiler/src/rust_method_registry.rs"
tests="$root/crates/compiler/src/tests/builtin_tests.rs"

# The primary parser must no longer special-case variable methods/fields.
! grep -Fq 'field == "elapsed"' "$primary"
! grep -Fq 'method_builtin' "$primary"
! grep -Fq 'self.tokens.get(self.pos) == Some(&ExprToken::Dot)' "$primary"

# One generic postfix path owns Rust-like member/method parsing.
grep -Fq 'pub(super) fn parse_postfix' "$postfix"
grep -Fq 'rust_method_registry::resolve(&member)' "$postfix"
grep -Fq 'Expr::Field {' "$postfix"
grep -Fq 'field: member' "$postfix"

# Method names must not become parser-level policy again.
! grep -Eq 'member[[:space:]]*==|member\.as_str\(\)|matches!\(member' "$postfix"
! grep -Fq 'resolve_pure_method' "$postfix"

# The semantic registry is the single name -> operation boundary.
grep -Fq 'pub(super) enum RustMethod' "$registry"
grep -Fq 'pub(super) fn resolve(name: &str) -> Option<RustMethod>' "$registry"
grep -Fq 'Ambient-authority APIs are intentionally absent' "$registry"

# Existing Rust-like regressions remain covered.
grep -Fq 'started.elapsed().as_nanos()' "$tests"
grep -Fq 'a64.sin() + 0.5f32 * a256.sin()' "$tests"

echo "generic semantic postfix resolver gate: ok"
