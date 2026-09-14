#!/bin/sh
set -eu

grep -q 'discover_sources' crates/compiler/src/source_loader/tree.rs
grep -q 'pub(crate) fn discover_sources' crates/compiler/src/source_discovery.rs
grep -q 'discover_source_paths' crates/native-build/src/incremental_build/mod.rs
grep -q 'snapshot_source_roots' crates/server/src/source_reload.rs
grep -q 'snapshot_source_roots(app)' crates/server/src/backend_support.rs
grep -q 'env_remove("RUSTFLAGS")' crates/native-build/src/rustc_backend/build.rs
grep -q 'env_remove("RUSTC_WRAPPER")' crates/native-build/src/rustc_backend/build.rs
grep -q '"--crate-type".into(), "cdylib".into()' crates/native-build/src/rustc_backend/build.rs
! grep -q -- '--extern' crates/native-build/src/rustc_backend/build.rs || grep -q '!args.iter().any(|arg| arg == "--extern")' crates/native-build/src/rustc_backend/build.rs
grep -Eq 'pub const CODEGEN_VERSION: &str = "rust-aot-[0-9]+-' crates/compiler/src/codegen/mod.rs
grep -Eq 'pub const EXECUTABLE_IR_VERSION: u16 = (1[2-9]|[2-9][0-9]+);' crates/executable-ir/src/model.rs
printf '%s\n' 'Velran live source + std-only shard checks: PASS'
