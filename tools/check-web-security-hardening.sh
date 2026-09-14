#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "web-security-hardening: FAIL: $1" >&2; exit 1; }

grep -Fq 'valid_origin_form_target(target)' crates/server/src/http_io.rs || fail 'request-target validator is not enforced at HTTP parse boundary'
grep -Fq 'decoded == b"." || decoded == b".." || decoded.first() == Some(&b' crates/server/src/request_target_security.rs || fail 'dot/hidden segments are not fail-closed'
grep -Fq "matches!(value, b'/' | b'\\\\')" crates/server/src/request_target_security.rs || fail 'encoded separators are not rejected'
grep -Fq 'has_unsafe_path_segments(path)' crates/compiler/src/outbound_calls.rs || fail 'compiler outbound path hardening missing'
grep -Fq 'has_unsafe_segments(value)' crates/integrations/src/egress_capability.rs || fail 'runtime outbound path hardening missing'
grep -Fq 'DNS answer outside target CIDR' crates/integrations/src/https_client.rs || fail 'DNS rebinding/CIDR answer validation missing'
grep -Fq 'connected peer IP denied' crates/integrations/src/https_client.rs || fail 'connected peer IP revalidation missing'
grep -Fq 'DebugRustcReproEnabled' crates/server/src/production_security.rs || fail 'production rustc repro prohibition missing'
grep -Fq 'response.headers().count() > 64' crates/server/src/http_io.rs || fail 'response header-count cap missing'
grep -Fq '__Host-velran_session' crates/server/src/session_cookie.rs || fail '__Host session cookie missing'
grep -Fq 'HttpOnly' crates/server/src/session_cookie.rs || fail 'HttpOnly session cookie missing'
grep -Fq 'SameSite=' crates/server/src/session_cookie.rs || fail 'SameSite session policy missing'
grep -Fq 'SECURITY_POLICY_VERSION: &str = "security-3-deployment-build-identity"' crates/language-core/src/artifact_versions.rs || fail 'security policy version not bumped'
echo 'web-security-hardening: PASS'
