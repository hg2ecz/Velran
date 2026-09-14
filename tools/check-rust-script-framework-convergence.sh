#!/bin/sh
set -eu
fail() { echo "Rust-script/framework convergence check: $*" >&2; exit 1; }

# Script UX is lifecycle/tooling, not a second application language.
grep -q 'debug_rustc_repro' crates/server/src/server_config_file.rs || fail 'reproducible direct-rustc diagnostics missing'
grep -q 'poll_interval_ms' crates/server/src/server_config_file.rs || fail 'live source polling configuration missing'
grep -q 'debounce_ms' crates/server/src/server_config_file.rs || fail 'source debounce configuration missing'

# Generated application shards remain std-only and capability isolated.
! grep -R -nE 'args\.(push|extend).*--extern|Command.*--extern' crates/native-build/src/rustc_backend >/dev/null || fail 'generated shard build path gained --extern dependency'
grep -q 'pub fn uses_host_api' crates/executable-ir/src/native_scalar.rs || fail 'verified host-effect classification missing'
grep -q 'body.uses_host_api()' crates/compiler/src/codegen/emit.rs || fail 'HostApi is not gated by verified effects'

# Local string/dictionary work must remain native-lowerable.
grep -q 'StringDict' crates/executable-ir/src/native_scalar.rs || fail 'native StringDict representation missing'
grep -q 'DictNew' crates/executable-ir/src/native_scalar.rs || fail 'BTreeMap constructor native lowering missing'
grep -q 'StringDictSet' crates/executable-ir/src/native_scalar.rs || fail 'native dictionary mutation lowering missing'
grep -Fq 'StringDict(Arc<BTreeMap<Arc<str>, Arc<str>>>)' crates/compiler/src/codegen/emit.rs || fail 'generated Rust BTreeMap representation missing'
grep -q 'string_dict_set' crates/compiler/src/codegen/lower.rs || fail 'generated dictionary set lowering missing'

# User surface follows Rust where semantics are equivalent.
grep -q 'BTreeMap::new' crates/compiler/src/expression_primary.rs || fail 'Rust-like map constructor missing'
grep -q 'contains_key' crates/compiler/src/expression_primary.rs || fail 'Rust-like map lookup missing'
test -f crates/compiler/src/expression_postfix.rs || fail 'general Rust-like postfix method parser missing'
if grep -R -nE '(^|[^.[:alnum:]_])(trim|trimStart|trimEnd|lower|upper|contains|startsWith|endsWith|replace|stringLen|repeat|dict|containsKey)\(' examples start_examples --include='*.vrn' >/dev/null; then
  fail 'legacy duplicate string surface remains in runnable examples'
fi
if grep -R -nE '(^|[^[:alnum:]_])split\(' examples start_examples --include='*.vrn' >/dev/null; then
  fail 'unbounded split remains in runnable examples'
fi

# Hidden path segments are rejected below the HTTP serving layer too.
grep -q "p.starts_with('.')" crates/language-core/src/values.rs || fail 'shared hidden-path rejection missing'

printf '%s\n' 'Rust-script/framework convergence verification passed'
