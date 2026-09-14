#!/bin/sh
set -eu

grep -q 'debug_compile_errors: bool' crates/server/src/server_config_file.rs
grep -q 'debug_compile_errors: Option<bool>' crates/server/src/server_config_file.rs
grep -q '"--debug-compile-errors"' crates/server/src/cli_overrides.rs
grep -q 'render_debug(&self, app_entry: &Path)' crates/compiler/src/diagnostics.rs
grep -q 'terminal_safe' crates/compiler/src/diagnostics.rs
grep -q 'serving last valid generation' crates/server/src/source_reload.rs
grep -q 'DebugCompileErrorsEnabled' crates/server/src/production_security.rs
grep -q 'source_reload.debug_compile_errors || domain.reload.debug_compile_errors' crates/server/src/production_security.rs

echo 'Velran debug diagnostics checks passed'
