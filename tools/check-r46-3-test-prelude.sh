#!/usr/bin/env bash
set -euo pipefail
file="crates/server/src/main_tests/test_support.rs"
grep -Fq 'pub(super) use crate::operations::serve_health_endpoint;' "$file" || {
  echo 'R46.3: server test prelude must import serve_health_endpoint directly' >&2
  exit 1
}
if grep -Fq 'use operations::{install_panic_logging_hook, serve_health_endpoint};' crates/server/src/main.rs; then
  echo 'R46.3: serve_health_endpoint must not return to production root import surface' >&2
  exit 1
fi
echo 'R46.3 test-prelude guard: OK'
