#!/bin/sh
set -eu

fail() { echo "rustc-first security check: FAIL: $*" >&2; exit 1; }
pass() { echo "rustc-first security and diagnostics: PASS"; }

# The native shard must remain safe Rust except for the compiler-owned ABI shim.
grep -Fq 'source.push_str("pub mod implementation {\n#![forbid(unsafe_code)]' crates/compiler/src/codegen/emit.rs \
  || fail 'generated implementation no longer forbids unsafe code'

# Ambient Cargo/Rust injection points must stay removed from the direct rustc invocation.
for var in RUSTFLAGS RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER RUSTDOCFLAGS CARGO_ENCODED_RUSTFLAGS CARGO_HOME; do
  grep -Fq ".env_remove(\"$var\")" crates/native-build/src/rustc_backend/command.rs \
    || fail "isolated rustc invocation no longer removes $var"
done

# rustc stderr must be captured and carried as a first-class diagnostic.
grep -Fq '.output()' crates/native-build/src/rustc_backend/command.rs \
  || fail 'rustc output is not captured'
grep -Fq 'diagnostics: String' crates/native-build/src/rustc_backend/error.rs \
  || fail 'BuildError no longer carries rustc diagnostics'
grep -Fq 'pub fn rustc_diagnostics(&self)' crates/native-build/src/rustc_backend/error.rs \
  || fail 'rustc diagnostics accessor missing'

# The pre-rustc source security gate must execute for both file and in-memory compilation.
grep -Fq 'rust_surface_security::validate(&source)' crates/compiler/src/compile_api.rs \
  || fail 'in-memory source skips Rust-surface security gate'
grep -Fq 'rust_surface_security::validate(&source)' crates/compiler/src/source_loader/tree.rs \
  || fail 'file source skips Rust-surface security gate'
grep -Fq 'SEC-RUST-001' crates/compiler/src/rust_surface_security.rs \
  || fail 'Rust-surface security diagnostic missing'
grep -Fq '("panic!", "panic/abort macros")' crates/compiler/src/rust_surface_security.rs \
  || fail 'panic/abort escape macros are not blocked before rustc'

# Inherent impls must be discovered then lowered through verified pure compute; raw impl bodies
# must never be passed through as an alternate Rust execution surface.
grep -Fq 'register_inherent_impls' crates/compiler/src/compile_pipeline.rs \
  || fail 'inherent impl signatures are not registered'
grep -Fq 'lower_inherent_impls' crates/compiler/src/compile_pipeline.rs \
  || fail 'inherent impl bodies are not lowered through the verified path'
grep -Fq 'parse_pure_functions(&synthetic' crates/compiler/src/inherent_impl.rs \
  || fail 'inherent method body no longer reuses verified pure-function lowering'
grep -Fq 'immutable `&self` receiver' crates/compiler/src/inherent_impl.rs \
  || fail 'mutable/owned self rejection missing'
grep -Fq 'trait impls are not supported yet' crates/compiler/src/inherent_impl.rs \
  || fail 'trait impls are not fail-closed'

# Non-production developer diagnostics must be HTML escaped and domain scoped.
grep -Fq 'html_escape(&text)' crates/server/src/dev_compile_error.rs \
  || fail 'developer compiler error page is not escaped'
grep -Fq 'BTreeMap<String, String>' crates/server/src/dev_compile_error.rs \
  || fail 'developer compiler diagnostics are not domain scoped'
grep -Fq 'source_reload_rustc_diagnostics' crates/server/src/source_reload.rs \
  || fail 'rustc diagnostics are not written to the server error log'
grep -Fq 'startup_rustc_diagnostics' crates/server/src/main.rs \
  || fail 'startup rustc diagnostics are not written to the server error log'
grep -Fq 'forbids detailed compiler diagnostics' crates/server/src/production_security.rs \
  || fail 'production no longer rejects detailed compiler diagnostics'

pass
