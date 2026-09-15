#!/bin/sh
set -eu

fail() {
  echo "debug diagnostics check: $*" >&2
  exit 1
}

require_grep() {
  pattern="$1"
  file="$2"
  message="$3"
  grep -Fq -- "$pattern" "$file" || fail "$message ($file)"
}

require_grep 'debug_compile_errors: bool' crates/server/src/server_config_file.rs 'effective debug compiler diagnostics flag missing'
require_grep 'debug_compile_errors: Option<bool>' crates/server/src/server_config_file.rs 'optional config-layer debug compiler diagnostics flag missing'
require_grep '"--debug-compile-errors"' crates/server/src/cli_overrides.rs 'CLI debug compiler diagnostics override missing'
require_grep 'render_debug(&self, app_entry: &Path)' crates/compiler/src/diagnostics.rs 'debug diagnostic renderer missing'
require_grep 'terminal_safe' crates/compiler/src/diagnostics.rs 'terminal-safe diagnostic rendering missing'
require_grep 'serving last valid generation' crates/server/src/source_reload.rs 'rolling reload must retain the last valid generation on candidate failure'
require_grep 'DebugCompileErrorsEnabled' crates/server/src/production_security.rs 'production policy debug diagnostics rejection missing'
require_grep 'domain.reload.debug_compile_errors' crates/server/src/production_security.rs 'production policy must validate per-domain reload debug diagnostics'
require_grep 'domain.reload.enabled && domain.reload.mode != ReloadMode::Rolling' crates/server/src/production_security.rs 'production policy must distinguish transactional rolling reload from unsafe development reload'

printf '%s\n' 'Velran debug diagnostics checks passed'
