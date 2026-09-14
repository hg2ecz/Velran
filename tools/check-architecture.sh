#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

fail() { echo "architecture: $*" >&2; exit 1; }

internal_deps() {
  cargo_toml=$1
  grep -o 'path = "../[^"]*"' "$cargo_toml" 2>/dev/null \
    | sed 's#.*path = "../##; s#"##' \
    | LC_ALL=C sort -u || true
}

check_allowed_deps() {
  crate=$1
  allowed=" $2 "
  cargo_toml="crates/$crate/Cargo.toml"
  [ -f "$cargo_toml" ] || fail "missing $cargo_toml"
  for dep in $(internal_deps "$cargo_toml"); do
    case "$allowed" in
      *" $dep "*) : ;;
      *) fail "$crate must not depend on internal crate $dep (allowed:$allowed)" ;;
    esac
  done
}

max_lines() {
  file=$1
  max=$2
  lines=$(wc -l < "$file" | tr -d ' ')
  [ "$lines" -le "$max" ] || fail "$file grew to $lines lines (architecture budget $max)"
}

# Internal dependency direction. These are allow-lists, so a newly introduced
# cross-layer dependency fails until its architectural role is reviewed.
check_allowed_deps language-core ""
check_allowed_deps data ""
check_allowed_deps integrations ""
check_allowed_deps storage ""
check_allowed_deps observability ""
check_allowed_deps auth "data"
check_allowed_deps compiler "language-core executable-ir runtime-abi"
check_allowed_deps executable-ir "language-core"
check_allowed_deps runtime-abi ""
check_allowed_deps native-build "compiler executable-ir runtime-abi"
check_allowed_deps native-runtime "native-build executable-ir observability runtime-abi"
check_allowed_deps runtime "language-core data compiler"
check_allowed_deps cli "auth compiler data"
check_allowed_deps server "native-runtime auth compiler data integrations language-core runtime runtime-abi storage observability"

# The server is the composition root for the trusted outbound adapter.
# `runtime` remains transport-agnostic; only server may bind runtime::OutboundRuntime
# to the SSRF-hardened integrations crate.

# Production error APIs must stay typed. Trait-object transport abstractions are
# allowed, but dynamic error erasure is not.
boxed_errors=$(grep -RIl --include='*.rs' -E 'Box<dyn (std::error::Error|Error)>' crates 2>/dev/null || true)
[ -z "$boxed_errors" ] || fail "boxed error API found; use a typed orchestration error: $boxed_errors"

string_errors=$(grep -RIl --include='*.rs' -E 'Result<.*,[[:space:]]*String>' crates 2>/dev/null || true)
[ -z "$string_errors" ] || fail "String error API found; use a typed error: $string_errors"

# Public façades should stay façades. These budgets are regression alarms, not
# invitations to split cohesive code solely to satisfy a line count.
max_lines crates/server/src/main.rs 600
max_lines crates/server/src/operations.rs 280
max_lines crates/server/src/request_input.rs 160
max_lines crates/compiler/src/lib.rs 200
max_lines crates/compiler/src/expression.rs 500
max_lines crates/compiler/src/source_syntax.rs 230
max_lines crates/compiler/src/sql_syntax.rs 110
max_lines crates/executable-ir/src/lib.rs 50
max_lines crates/executable-ir/src/model.rs 100
max_lines crates/executable-ir/src/verifier.rs 320
max_lines crates/executable-ir/src/shard_planner/mod.rs 100
max_lines crates/compiler/src/codegen/mod.rs 140
max_lines crates/runtime-abi/src/lib.rs 40
max_lines crates/native-build/src/artifact_format/mod.rs 190
max_lines crates/native-runtime/src/artifact_loader/mod.rs 100
max_lines crates/native-runtime/src/artifact_loader/unix.rs 80
max_lines crates/native-runtime/src/runtime_generation/mod.rs 130
max_lines crates/native-build/src/build_cache/mod.rs 300
max_lines crates/native-build/src/rustc_backend/mod.rs 140
max_lines crates/native-build/src/rustc_backend/build.rs 180
max_lines crates/native-build/src/rustc_backend/abi_shim.rs 80
max_lines crates/native-build/src/incremental_build/mod.rs 300
max_lines crates/native-build/src/incremental_build/tests.rs 120
max_lines crates/native-runtime/src/application_runtime/mod.rs 60
max_lines crates/native-runtime/src/application_runtime/engine.rs 220
max_lines crates/native-runtime/src/application_runtime/error.rs 60
max_lines crates/native-runtime/src/application_runtime/worker_policy.rs 100
max_lines crates/native-build/src/incremental_build/source_tracking.rs 160
max_lines crates/native-runtime/src/runtime_generation/generation.rs 100
max_lines crates/runtime-abi/src/result.rs 80
max_lines crates/auth/src/lib.rs 380
max_lines crates/runtime/src/lib.rs 120
max_lines crates/runtime/src/response.rs 40

# Known one-file hotspots are frozen against further growth until their planned
# responsibility-based decomposition is complete.
max_lines crates/data/src/lib.rs 120
max_lines crates/data/src/database.rs 320
max_lines crates/data/src/sql.rs 360
max_lines crates/data/src/redis_store.rs 260
max_lines crates/data/src/types.rs 120
max_lines crates/data/src/error.rs 100
max_lines crates/language-core/src/lib.rs 80
max_lines crates/language-core/src/ast.rs 450
max_lines crates/language-core/src/values.rs 280
max_lines crates/language-core/src/web_types.rs 140
max_lines crates/language-core/src/schema.rs 100
max_lines crates/language-core/src/query.rs 80
max_lines crates/language-core/src/routing.rs 100
max_lines crates/language-core/src/program.rs 100
max_lines crates/language-core/src/config.rs 80
max_lines crates/language-core/src/error.rs 80
max_lines crates/data/src/migrations/mod.rs 80
max_lines crates/data/src/migrations/source.rs 240
max_lines crates/data/src/migrations/service.rs 190
max_lines crates/data/src/migrations/database.rs 150
max_lines crates/data/src/migrations/locking.rs 120
max_lines crates/data/src/migrations/history.rs 80
max_lines crates/data/src/migrations/types.rs 80
max_lines crates/data/src/migrations/error.rs 80
max_lines crates/data/src/migrations/error_display.rs 80
max_lines crates/integrations/src/lib.rs 80
max_lines crates/integrations/src/egress.rs 300
max_lines crates/integrations/src/secrets.rs 160
max_lines crates/integrations/src/https_client.rs 340
max_lines crates/integrations/src/error.rs 80
max_lines crates/storage/src/lib.rs 80
max_lines crates/storage/src/filesystem.rs 450
max_lines crates/storage/src/upload.rs 240
max_lines crates/storage/src/image.rs 180
max_lines crates/observability/src/lib.rs 80
max_lines crates/observability/src/events.rs 180
max_lines crates/observability/src/metrics.rs 360
max_lines crates/observability/src/logging.rs 320
max_lines crates/observability/src/error.rs 80

# Stable façade surfaces created by the completed auth extractions.
grep -q '^mod session;$' crates/auth/src/lib.rs || fail 'auth façade must own session as a private module'
grep -q '^mod local_user;$' crates/auth/src/lib.rs || fail 'auth façade must own local_user as a private module'
grep -q '^pub use session::{RedisSessionStore, SessionBackend, SessionFlash, SessionSnapshot, SessionStore};$' crates/auth/src/lib.rs \
  || fail 'auth façade must preserve the session public surface through explicit re-exports'
grep -q '^pub use local_user::{LocalUserAuth, LocalUserStore};$' crates/auth/src/lib.rs \
  || fail 'auth façade must preserve the local-user public surface through explicit re-exports'

# Browser security policy stays platform-owned and least-privilege.
grep -q '^mod security_headers;$' crates/server/src/main.rs \
  || fail 'server must keep browser security-header generation in a dedicated module'
grep -q "connect-src 'none'" crates/server/src/security_headers.rs \
  || fail 'generated CSP must not grant ambient browser connect authority'
if grep -R -q "img-src 'self' data:\|connect-src 'self'" crates/server/src --include='*.rs'; then
  fail 'server must not restore global data-image or browser-connect CSP authority'
fi

printf '%s\n' 'architecture verification passed'

# R48 module resolution stays a small compiler-owned boundary.
max_lines crates/compiler/src/source_loader.rs 180
max_lines crates/compiler/src/source_loader/tree.rs 220
max_lines crates/compiler/src/source_loader/reexports.rs 140
max_lines crates/compiler/src/module_namespace.rs 80

# Typed response metadata stays closed over known header names and validated values.
test -f crates/server/src/response_headers.rs || fail 'missing typed response-header module'
grep -q '^mod response_headers;$' crates/server/src/main.rs \
  || fail 'server must own response_headers explicitly'
grep -q '^    headers: Vec<ResponseHeader>,$' crates/server/src/http_io.rs \
  || fail 'response headers must remain private typed ResponseHeader values'
if grep -q '^    pub(super) headers: Vec<ResponseHeader>,' crates/server/src/http_io.rs; then
  fail 'response header storage must not become directly writable outside http_io'
fi
grep -q 'response contains invalid typed HTTP metadata' crates/server/src/http_io.rs \
  || fail 'HTTP writer must fail closed on invalid response metadata'
printf '%s\n' 'typed HTTP response metadata verification passed'
if grep -R -q --include='*.rs' --exclude='http_io.rs' '\.headers\.push' crates/server/src; then
  fail 'server modules must add response headers through the typed Response::push_header boundary'
fi

# Cross-crate public facade required by the server composition root.
grep -q '^pub use http_metadata::{ContentDisposition, FileName, MediaType};$' crates/language-core/src/lib.rs \
  || fail 'language-core facade must re-export typed HTTP metadata'
grep -q '^pub use production_policy::ProductionPolicy;$' crates/language-core/src/lib.rs \
  || fail 'language-core facade must re-export ProductionPolicy'
grep -Eq '^pub use effect::\{Effect, EffectClass\};$|^pub use effect::Effect;$' crates/language-core/src/lib.rs \
  || fail 'language-core facade must re-export Effect'
grep -q '^pub use outbound::{OutboundFuture, OutboundOutcome, OutboundRuntime};$' crates/runtime/src/lib.rs \
  || fail 'runtime facade must re-export outbound capability types and bounded usage metadata'
printf '%s\n' 'cross-crate facade verification passed'

# Rust source release archives must not force epoch mtimes. Doing so can make
# Cargo reuse stale .rmeta files when a source archive is extracted over a
# workspace that still contains target/ artifacts.
if grep -q -- "--mtime=['\"]\?@0" tools/package-release.sh; then
  fail 'source release packaging must not force Unix-epoch mtimes'
fi
grep -q 'ARCHIVE_EPOCH=${SOURCE_DATE_EPOCH:-$(date +%s)}' tools/package-release.sh \
  || fail 'source release packaging must default to a fresh archive timestamp'
grep -q 'source archive timestamp must be >= 2000-01-01' tools/package-release.sh \
  || fail 'source release packaging must reject pathological old mtimes'
printf '%s\n' 'source archive Cargo-freshness verification passed'

# Supply-chain authority is explicit release state. A dependency edge or
# capability escalation must invalidate the reviewed supply-chain lock.
test -x tools/supply-chain-verify.sh || fail 'missing executable supply-chain verifier'
test -x tools/supply-chain-lock.sh || fail 'missing executable supply-chain lock refresher'
test -f SUPPLY-CHAIN-CAPABILITIES.txt || fail 'missing direct dependency capability inventory'
test -f VELRAN-SUPPLY-CHAIN.lock || fail 'missing reviewed supply-chain lock envelope'
grep -q '^provenance crates.io-checksummed-only$' VELRAN-SUPPLY-CHAIN.lock \
  || fail 'supply-chain provenance must remain checksummed crates.io only'
grep -q '^capability-model direct-external-dependency-edge$' VELRAN-SUPPLY-CHAIN.lock \
  || fail 'supply-chain capability review model changed without architecture review'
grep -q '^./tools/supply-chain-verify.sh$' tools/dependency-audit.sh \
  || fail 'dependency audit must verify supply-chain capability/provenance state first'
grep -q '^./tools/supply-chain-verify.sh$' tools/package-release.sh \
  || fail 'release packaging must fail closed on stale supply-chain state'
grep -q 'cargo metadata --locked --format-version 1' tools/supply-chain-lock.sh \
  || fail 'supply-chain envelope generation must validate Cargo.lock against manifests'
grep -q 'cargo metadata --locked --format-version 1' tools/package-release.sh \
  || fail 'release packaging must validate Cargo.lock against manifests'
grep -q 'cargo metadata --locked --format-version 1' verify.sh \
  || fail 'verification must validate Cargo.lock against manifests before supply-chain envelope'
printf '%s\n' 'supply-chain capability/provenance architecture verification passed'

# Advanced resource safety II: compressed request/upstream bodies stay outside
# the trusted application boundary until a future bounded decompression API exists.
grep -q 'MAX_VELRAN_STATUS_BODY_BYTES: usize = 256 \* 1024' crates/integrations/src/https_client.rs \
  || fail 'Velran outbound status calls must retain a hard response-body cap'
grep -q 'name == "content-encoding" && !value.eq_ignore_ascii_case("identity")' crates/integrations/src/http_response.rs \
  || fail 'upstream compressed responses must fail closed'
grep -q '"content-encoding" => {' crates/server/src/http_io.rs \
  || fail 'compressed inbound request bodies must remain explicitly guarded'
grep -q 'remaining_external_io_bytes: u64' crates/server/src/native_host.rs \
  || fail 'native host bridge must retain cumulative external I/O accounting'
printf '%s\n' 'advanced resource safety II verification passed'


# Security monitoring core: redaction cannot be forged, structured security
# telemetry stays payload-free, and burst alerts must use the redacted event API.
grep -q 'Redact' crates/language-core/src/builtin.rs \
  || fail 'language core must retain the trusted redact builtin'
grep -q 'DataSensitivity::Redacted' crates/compiler/src/expression_security.rs \
  || fail 'compiler must retain Redacted<T> metadata for redact(...) results'
if grep -q 'unwrap_generic(raw, "Redacted")' crates/compiler/src/type_resolution.rs; then
  fail 'Redacted<T> must not become a user-forgeable type annotation'
fi
grep -q 'subject: "\[redacted\]"' crates/observability/src/logging.rs \
  || fail 'structured security events must redact subject identity'
grep -q 'source: "\[redacted\]"' crates/observability/src/logging.rs \
  || fail 'structured security events must redact source identity'
grep -q 'mod security_alerts;' crates/server/src/main.rs \
  || fail 'server must own the security alert policy boundary explicitly'
grep -q 'threshold_exceeded' crates/server/src/security_alerts.rs \
  || fail 'security monitoring must retain threshold-based alert emission'
printf '%s\n' 'security monitoring core verification passed'
