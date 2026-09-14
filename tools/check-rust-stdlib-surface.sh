#!/bin/sh
set -eu
fail() { echo "rust stdlib surface check: $*" >&2; exit 1; }

# Public examples should use Rust-like method/constructor syntax where semantics match.
if grep -R -nE '(^|[^.[:alnum:]_])(trim|trimStart|trimEnd|lower|upper|contains|startsWith|endsWith|replace|stringLen|repeat|dict|containsKey)\(' examples start_examples --include='*.vrn' >/dev/null; then
  fail 'legacy duplicate string/dictionary surface remains in examples'
fi
grep -q 'text\.trim()' examples/string-operations/main.vrn || fail 'Rust-like trim example missing'
grep -q 'cleaned\.chars().count()' examples/string-operations/main.vrn || fail 'Rust-like char count example missing'
grep -q 'BTreeMap::new()' examples/string-dict/main.vrn || fail 'Rust-like BTreeMap constructor missing'
grep -q 'headers\.contains_key(key)' examples/string-dict/main.vrn || fail 'Rust-like contains_key missing'

# Legacy forms must fail with migration diagnostics instead of silently coexisting.
grep -q 'legacy `.*(...)` syntax is not supported' crates/compiler/src/expression_primary.rs || fail 'legacy string method rejection missing'
grep -q 'BTreeMap::new' crates/compiler/src/expression_primary.rs || fail 'BTreeMap::new parser lowering missing'
grep -q 'contains_key' crates/compiler/src/expression_primary.rs || fail 'contains_key parser lowering missing'

# Dot-prefixed path components are denied at the shared image-reference boundary as well.
grep -q "p.starts_with('.')" crates/language-core/src/values.rs || fail 'hidden image path segment rejection missing'

printf '%s\n' 'Rust stdlib surface/security convergence verification passed'
grep -q 'pub fn uses_host_api' crates/executable-ir/src/native_scalar.rs || fail 'verified scalar host-effect classification missing'
grep -q 'body.uses_host_api()' crates/compiler/src/codegen/emit.rs || fail 'codegen does not gate HostApi by verified effects'
grep -q 'rust-aot-50-release-stabilization' crates/compiler/src/codegen/mod.rs || fail 'codegen cache version not bumped'
