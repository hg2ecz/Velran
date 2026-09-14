#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

fail() {
  echo "iteration1-release-hardening: FAIL: $*" >&2
  exit 1
}

critical_files="crates/server/src/auth_http.rs
crates/server/src/backend_support.rs
crates/server/src/source_reload_candidate.rs
crates/server/src/startup_transport.rs
crates/server/src/web_security.rs"

# Security boundaries must not depend on panic-only control flow. These files
# process attacker-controlled HTTP metadata, authentication state, or hot-reload
# configuration and therefore must fail closed with explicit branches.
for file in $critical_files; do
  if grep -nE '\.(unwrap|expect)\(' "$file" >/dev/null; then
    grep -nE '\.(unwrap|expect)\(' "$file" >&2 || true
    fail "$file contains unwrap()/expect() in a security-critical production path"
  fi
done

grep -Fq 'map.remove("_csrf")' crates/server/src/auth_http.rs \
  || fail 'login form must extract the required CSRF field explicitly'
grep -Fq 'map.remove("username")' crates/server/src/auth_http.rs \
  || fail 'login form must extract the required username explicitly'
grep -Fq 'let Some(ldap) = auth.ldap.as_ref() else' crates/server/src/auth_http.rs \
  || fail 'LDAP fallback must fail closed when the configured backend disappears'
grep -Fq 'let Some(local) = auth.local.as_ref() else' crates/server/src/auth_http.rs \
  || fail 'recovery-code flow must fail closed when the local backend is unavailable'

grep -Fq 'if let Some(public_cache) = route.public_cache.as_ref()' crates/server/src/source_reload_candidate.rs \
  || fail 'source-reload cache policy must use explicit optional handling'
grep -Fq '.is_some_and(|public_cache| public_cache.ttl_secs > cache_cli.max_ttl_secs)' crates/server/src/backend_support.rs \
  || fail 'domain reload cache policy must avoid panic-only optional handling'

grep -Fq 'let Some(value) = forwarded else' crates/server/src/web_security.rs \
  || fail 'proxy-header parsing must fail closed when Forwarded is absent'
grep -Fq 'let public_host = tls.public_host.clone()?;' crates/server/src/startup_transport.rs \
  || fail 'redirect listener must not panic on a missing public host invariant'

# Preserve the larger web security and CPU/cache identity contracts while
# keeping this final gate independent from historical text-shape assertions.
./tools/check-web-security-hardening.sh >/dev/null
./tools/check-target-cpu.sh >/dev/null
./tools/check-architecture.sh >/dev/null

[ "$(wc -l < crates/compiler/src/action_statements.rs | tr -d ' ')" -le 500 ]   || fail 'action statement parser exceeded its clean-code size budget'
[ "$(wc -l < crates/compiler/src/schema_declarations.rs | tr -d ' ')" -le 450 ]   || fail 'schema declaration facade exceeded its clean-code size budget'
[ "$(wc -l < crates/compiler/src/expression.rs | tr -d ' ')" -le 280 ]   || fail 'expression facade exceeded its clean-code size budget'
[ "$(wc -l < crates/server/src/auth_http.rs | tr -d ' ')" -le 380 ]   || fail 'authentication HTTP boundary exceeded its clean-code size budget'

test -f crates/compiler/src/schema_declarations/forms.rs   || fail 'form-schema parsing responsibility extraction is missing'
test -f crates/compiler/src/expression_type.rs   || fail 'expression type-inference responsibility extraction is missing'
test -f crates/compiler/src/tests/module_namespace_visibility_compile_tests.rs   || fail 'module visibility/re-export regression tests were not split into a cohesive module'

echo 'iteration1-release-hardening: PASS'
