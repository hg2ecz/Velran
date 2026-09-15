#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
./tools/check-architecture.sh

fail() { echo "clean-structure: $*" >&2; exit 1; }
max_lines() {
  file=$1
  max=$2
  lines=$(wc -l < "$file" | tr -d ' ')
  [ "$lines" -le "$max" ] || fail "$file grew to $lines lines (limit $max); extract a cohesive responsibility instead of growing the god file"
}

compact_contains() {
  file=$1
  needle=$2
  normalized_needle=$(printf '%s' "$needle" | sed 's/,}/}/g')
  tr -d '[:space:]' < "$file" | sed 's/,}/}/g' | grep -Fq "$normalized_needle"
}

test -f crates/server/src/cli.rs || fail 'missing server CLI module'
test -f crates/server/src/cli_config_apply.rs || fail 'missing server config-application module'
test -f crates/server/src/cli_scan.rs || fail 'missing server CLI bootstrap scanner module'
test -f crates/server/src/cli_overrides.rs || fail 'missing server CLI override module'
test -f crates/server/src/cli_finalize.rs || fail 'missing server CLI finalization module'
test -f crates/server/src/startup.rs || fail 'missing server startup/lifecycle module'
test -f crates/server/src/http_dispatch.rs || fail 'missing server HTTP dispatch module'
test -f crates/server/src/connection.rs || fail 'missing server connection/request loop module'
test -f crates/server/src/connection_dispatch.rs || fail 'missing server connection dispatch module'
test -f crates/server/src/connection_finalize.rs || fail 'missing server response finalization module'
test -f crates/server/src/server_errors.rs || fail 'missing server typed error module'
test -f crates/server/src/server_errors/auth_setup.rs || fail 'missing server auth typed error module'
test -f crates/server/src/server_errors/policy_config.rs || fail 'missing server policy/resource typed error module'
test -f crates/server/src/server_errors/cli.rs || fail 'missing server CLI typed error module'
test -f crates/server/src/server_config_file.rs || fail 'missing server config/domain module'
test -f crates/server/src/bootstrap_config.rs || fail 'missing server bootstrap/config helper module'
test -f crates/server/src/auth_setup.rs || fail 'missing server auth setup module'
test -f crates/server/src/static_delivery.rs || fail 'missing server static delivery module'
test -f crates/server/src/rate_limit.rs || fail 'missing server rate-limit module'
test -f crates/server/src/source_reload.rs || fail 'missing source reload supervisor module'
test -f crates/compiler/src/routes.rs || fail 'missing compiler route module'
test -f crates/compiler/src/routes/route_scanner.rs || fail 'missing compiler route declaration scanner module'
test -f crates/compiler/src/source_loader.rs || fail 'missing compiler source/module loader'
test -f crates/compiler/src/query_parser.rs || fail 'missing compiler query/SQL parser module'
test -f crates/compiler/src/template_declarations.rs || fail 'missing compiler template declaration module'
test -f crates/compiler/src/handler_parser.rs || fail 'missing compiler page/action handler parser module'
test -f crates/compiler/src/page_statements.rs || fail 'missing compiler page statement parser module'
test -f crates/compiler/src/action_statements.rs || fail 'missing compiler action statement parser module'
test -f crates/compiler/src/statement_helpers.rs || fail 'missing compiler shared statement helper module'
test -f crates/compiler/src/handler_types.rs || fail 'missing compiler handler semantic types module'
test -f crates/compiler/src/html_template.rs || fail 'missing compiler HTML template parser module'
test -f crates/runtime/src/domain_values.rs || fail 'missing runtime domain validation module'
test -f crates/runtime/src/execution_context.rs || fail 'missing runtime execution context module'
test -f crates/runtime/src/request_binding.rs || fail 'missing runtime route/input binding module'
test -f crates/runtime/src/errors.rs || fail 'missing runtime typed error module'

grep -q '^mod cli;$' crates/server/src/main.rs || fail 'server main must own the cli module explicitly'
grep -q '^mod cli_config_apply;$' crates/server/src/main.rs || fail 'server main must own cli_config_apply explicitly'
grep -q '^mod startup;$' crates/server/src/main.rs || fail 'server main must own the startup module explicitly'
grep -q '^mod http_dispatch;$' crates/server/src/main.rs || fail 'server main must own the HTTP dispatch module explicitly'
grep -q '^mod server_errors;$' crates/server/src/main.rs || fail 'server main must own the typed error module explicitly'
grep -q '^mod server_config_file;$' crates/server/src/main.rs || fail 'server main must own the server config/domain module explicitly'
grep -q '^mod source_reload;$' crates/server/src/main.rs || fail 'server main must own the source reload supervisor module explicitly'
grep -q '^mod bootstrap_config;$' crates/server/src/main.rs || fail 'server main must own bootstrap_config explicitly'
grep -q '^mod auth_setup;$' crates/server/src/main.rs || fail 'server main must own auth_setup explicitly'
grep -q '^mod static_delivery;$' crates/server/src/main.rs || fail 'server main must own static_delivery explicitly'
grep -q '^mod rate_limit;$' crates/server/src/main.rs || fail 'server main must own rate_limit explicitly'
grep -q '^mod connection;$' crates/server/src/main.rs || fail 'server main must own the connection module explicitly'
grep -q '^mod connection_dispatch;$' crates/server/src/main.rs || fail 'server main must own connection_dispatch explicitly'
grep -q '^mod connection_finalize;$' crates/server/src/main.rs || fail 'server main must own connection_finalize explicitly'
grep -q 'cli::parse_args()' crates/server/src/main.rs || fail 'server startup must delegate argument parsing to cli::parse_args'
grep -q 'startup::run(parsed)' crates/server/src/main.rs || fail 'server main must delegate lifecycle startup to startup::run'
grep -q 'connection_dispatch::dispatch_' crates/server/src/connection.rs || fail 'connection loop must delegate request body/application dispatch'
grep -q 'http_dispatch::dispatch(' crates/server/src/connection_dispatch.rs || fail 'buffered connection dispatch must delegate application routing to http_dispatch::dispatch'
grep -q 'connection_finalize::finalize_response' crates/server/src/connection.rs || fail 'connection loop must delegate response finalization'
grep -q '^mod routes;$' crates/compiler/src/lib.rs || fail 'compiler lib must own the routes module explicitly'
grep -q '^mod source_loader;$' crates/compiler/src/lib.rs || fail 'compiler lib must own source_loader explicitly'
grep -q '^mod query_parser;$' crates/compiler/src/lib.rs || fail 'compiler lib must own query_parser explicitly'
grep -q '^mod template_declarations;$' crates/compiler/src/lib.rs || fail 'compiler lib must own template_declarations explicitly'
grep -q '^mod handler_parser;$' crates/compiler/src/lib.rs || fail 'compiler lib must own handler_parser explicitly'
grep -q '^mod page_statements;$' crates/compiler/src/lib.rs || fail 'compiler lib must own page_statements explicitly'
grep -q '^mod action_statements;$' crates/compiler/src/lib.rs || fail 'compiler lib must own action_statements explicitly'
grep -q '^mod statement_helpers;$' crates/compiler/src/lib.rs || fail 'compiler lib must own statement_helpers explicitly'
grep -q '^mod handler_types;$' crates/compiler/src/lib.rs || fail 'compiler lib must own handler_types explicitly'
grep -q '^mod html_template;$' crates/compiler/src/lib.rs || fail 'compiler lib must own html_template explicitly'
grep -q '^mod compile_pipeline;$' crates/compiler/src/lib.rs || fail 'compiler facade must own compile_pipeline explicitly'
grep -q 'routes::parse_routes' crates/compiler/src/compile_pipeline.rs || fail 'compiler orchestration must delegate route parsing'
grep -q 'routes::validate_routes' crates/compiler/src/compile_pipeline.rs || fail 'compiler orchestration must delegate route validation'
grep -q '^mod domain_values;$' crates/runtime/src/lib.rs || fail 'runtime lib must own the domain validation module explicitly'
grep -q '^mod execution_context;$' crates/runtime/src/lib.rs || fail 'runtime lib must own execution_context explicitly'
grep -q '^mod request_binding;$' crates/runtime/src/lib.rs || fail 'runtime lib must own request_binding explicitly'
grep -q '^mod errors;$' crates/runtime/src/lib.rs || fail 'runtime lib must own typed errors explicitly'
if grep -q '^use super::\*;' crates/compiler/src/handler_types.rs crates/compiler/src/regex_types.rs crates/compiler/src/cache_safety.rs crates/compiler/src/builtin_registry.rs crates/compiler/src/arrays.rs crates/compiler/src/dicts.rs; then
  fail 'small compiler boundary modules must use explicit imports, not use super::*'
fi

max_lines crates/server/src/main.rs 2000
max_lines crates/server/src/cli.rs 40
max_lines crates/server/src/cli_scan.rs 80
max_lines crates/server/src/cli_overrides.rs 460
max_lines crates/server/src/cli_finalize.rs 340
max_lines crates/server/src/cli_config_apply.rs 420
max_lines crates/server/src/server_errors.rs 260
max_lines crates/server/src/server_errors/auth_setup.rs 100
max_lines crates/server/src/server_errors/policy_config.rs 150
max_lines crates/server/src/server_errors/cli.rs 180
max_lines crates/server/src/connection.rs 400
max_lines crates/server/src/connection_dispatch.rs 450
max_lines crates/server/src/connection_finalize.rs 150
max_lines crates/server/src/startup.rs 500
max_lines crates/server/src/http_dispatch.rs 650
max_lines crates/server/src/source_reload.rs 450
max_lines crates/server/src/bootstrap_config.rs 650
max_lines crates/server/src/auth_setup.rs 200
max_lines crates/server/src/static_delivery.rs 400
max_lines crates/server/src/rate_limit.rs 180
max_lines crates/compiler/src/lib.rs 1400
max_lines crates/compiler/src/template_declarations.rs 400
max_lines crates/compiler/src/handler_parser.rs 250
max_lines crates/compiler/src/page_statements.rs 400
max_lines crates/compiler/src/action_statements.rs 500
max_lines crates/compiler/src/statement_helpers.rs 350
max_lines crates/compiler/src/handler_types.rs 100
max_lines crates/compiler/src/html_template.rs 600
max_lines crates/compiler/src/routes.rs 760
max_lines crates/compiler/src/routes/route_scanner.rs 180
max_lines crates/runtime/src/lib.rs 120
max_lines crates/runtime/src/request_binding.rs 450
max_lines crates/runtime/src/execution_context.rs 250


if grep -q 'Box<dyn std::error::Error>' crates/server/src/tls_support.rs crates/server/src/server_config_file.rs; then
  fail 'TLS and server-config leaf modules must expose typed errors, not Box<dyn Error>'
fi
grep -q 'pub(super) enum TlsConfigError' crates/server/src/server_errors.rs || fail 'missing typed TlsConfigError'
grep -q 'pub(super) enum ServerConfigError' crates/server/src/server_errors.rs || fail 'missing typed ServerConfigError'
grep -q 'pub(super) enum SecretFileError' crates/server/src/server_errors.rs || fail 'missing typed SecretFileError'

grep -q '^pub enum ResourceLimitError' crates/server/src/resource_limits.rs || fail 'server resource-limits module must expose typed ResourceLimitError internally'
if grep -q 'pub fn apply(config: &ResourceLimitConfig) -> Result<(), String>' crates/server/src/resource_limits.rs; then
  fail 'server resource-limits apply API must not return String errors'
fi


if grep -q 'Box<dyn std::error::Error>' crates/server/src/auth_setup.rs crates/server/src/cli.rs; then
  fail 'auth setup and CLI leaf APIs must expose typed errors, not Box<dyn Error>'
fi
if grep -q 'Box<dyn std::error::Error>' crates/server/src/bootstrap_config.rs; then
  fail 'bootstrap config parser helpers must expose typed errors, not Box<dyn Error>'
fi
grep -q 'Result<StartupArgs, CliParseError>' crates/server/src/cli.rs || fail 'CLI parser must return typed CliParseError via StartupArgs'
grep -q 'Result<Option<LdapConfig>, AuthSetupError>' crates/server/src/auth_setup.rs || fail 'LDAP setup must return typed AuthSetupError'
grep -q 'Result<HashMap<String, RatePolicy>, RatePolicyConfigError>' crates/server/src/bootstrap_config.rs || fail 'rate-policy loader must return typed RatePolicyConfigError'
grep -q 'Result<ResourceProfiles, ResourceProfileConfigError>' crates/server/src/bootstrap_config.rs || fail 'resource-profile loader must return typed ResourceProfileConfigError'


# R11: typed runtime/cache leaf boundaries.
max_lines crates/server/src/server_errors/runtime_boundary.rs 160
grep -q 'Result<u64, ClockError>' crates/server/src/main.rs || fail 'unix clock helper must return typed ClockError'
grep -Rqs --include='*.rs' 'Result<Vec<u8>, UploadRuntimeError>' crates/server/src || fail 'upload descriptor assembly must return typed UploadRuntimeError'
grep -q 'Result<Option<CachedPage>, PublicCacheError>' crates/server/src/bootstrap_config.rs || fail 'public cache get must return typed PublicCacheError'
grep -q 'Result<(), PublicCacheError>' crates/server/src/bootstrap_config.rs || fail 'public cache mutation helpers must return typed PublicCacheError'
if sed -n '/impl PublicPageCache {/,/^}/p' crates/server/src/bootstrap_config.rs | grep -q 'Result<.*String>'; then
  fail 'public cache leaf API must not return String errors'
fi
if grep -q 'Result<(), Box<dyn std::error::Error>>' crates/server/src/main.rs; then
  fail 'main leaf signal/listener helpers must not return boxed errors'
fi
printf '%s\n' 'clean-structure verification passed'

# R12: typed backend/reload boundaries and truthful HTTP dispatch signatures.
max_lines crates/server/src/server_errors/backend.rs 180
max_lines crates/server/src/server_errors/source_reload.rs 120
grep -q 'Result<BoundListener, BackendSupportError>' crates/server/src/backend_support.rs || fail 'application listener binding must return typed BackendSupportError'
grep -q 'Result<HostingRuntime, BackendSupportError>' crates/server/src/backend_support.rs || fail 'hosting runtime construction must return typed BackendSupportError'
grep -q 'Result<(), SourceReloadError>' crates/server/src/source_reload.rs || fail 'source reload validation/cache boundary must return typed SourceReloadError'
if grep -q 'Box<dyn std::error::Error>' crates/server/src/backend_support.rs crates/server/src/source_reload.rs crates/server/src/connection_dispatch.rs crates/server/src/http_io.rs; then
  fail 'R12 backend/reload/dispatch/http leaf modules must not return boxed errors'
fi
if grep -q 'Result<.*, String>' crates/server/src/source_reload.rs; then
  fail 'source reload leaf APIs must not return String errors'
fi
grep -q ') -> DispatchOutcome' crates/server/src/connection_dispatch.rs || fail 'connection dispatch must expose its infallible DispatchOutcome API directly'
grep -q ') -> io::Result<()>' crates/server/src/http_io.rs || fail 'HTTP response writer must return io::Result directly'
printf '%s\n' 'R12 typed boundary verification passed'

# R13: keep config-file application out of the command-line override parser.
grep -q 'cli_config_apply::load(bootstrap.config_path.as_deref())' crates/server/src/cli.rs || fail 'CLI parser must delegate config-file application'
if grep -q 'read_server_config(path)' crates/server/src/cli.rs; then
  fail 'CLI parser must not absorb config-file application again'
fi
printf '%s\n' 'R13 CLI responsibility extraction verification passed'

# R14: keep top-level route declaration scanning separate from route semantics.
grep -q '^mod route_scanner;$' crates/compiler/src/routes.rs || fail 'route parser must own the route_scanner submodule explicitly'
grep -q 'route_scanner::top_level_route_declarations(source)' crates/compiler/src/routes.rs || fail 'route parser must delegate top-level declaration scanning'
if grep -q '^fn top_level_route_declarations' crates/compiler/src/routes.rs; then
  fail 'route parser must not absorb top-level declaration scanning again'
fi
if grep -q '^use super::\*;' crates/compiler/src/routes/route_scanner.rs; then
  fail 'route scanner must keep explicit dependencies'
fi
printf '%s\n' 'R14 route scanner responsibility extraction verification passed'

# R15: keep proxy/origin/CORS web-security policy out of the server entrypoint.
test -f crates/server/src/web_security.rs || fail 'missing server web-security policy module'
grep -q '^mod web_security;$' crates/server/src/main.rs || fail 'server main must own web_security explicitly'
if grep -q '^use web_security::\*;$' crates/server/src/main.rs; then
  fail 'server main must not wildcard-import web-security helpers'
fi
if grep -q '^fn effective_client_ip\|^fn effective_request_https\|^fn validate_browser_state_change\|^fn valid_cors_origin\|^fn cors_preflight\|^fn apply_cors_headers' crates/server/src/main.rs; then
  fail 'server main must not absorb proxy/origin/CORS policy again'
fi
if grep -q '^use super::\*;' crates/server/src/web_security.rs; then
  fail 'web-security policy module must keep explicit dependencies'
fi
max_lines crates/server/src/main.rs 1700
max_lines crates/server/src/web_security.rs 320
printf '%s\n' 'R15 web-security responsibility extraction verification passed'

# R16: keep schema-like declaration parsing out of the compiler façade.
test -f crates/compiler/src/schema_declarations.rs || fail 'missing compiler schema declaration module'
grep -q '^mod schema_declarations;$' crates/compiler/src/lib.rs || fail 'compiler root must own schema_declarations explicitly'
grep -q 'schema_declarations::parse_enums' crates/compiler/src/compile_pipeline.rs || fail 'compiler pipeline must delegate enum parsing'
grep -q 'schema_declarations::parse_models' crates/compiler/src/compile_pipeline.rs || fail 'compiler pipeline must delegate model parsing'
grep -q 'schema_declarations::parse_form_schemas' crates/compiler/src/compile_pipeline.rs || fail 'compiler pipeline must delegate form-schema parsing'
if grep -q '^fn parse_enums\|^fn parse_models\|^fn parse_form_schemas\|^fn parse_validation_rule\|^fn validate_form_rules' crates/compiler/src/lib.rs; then
  fail 'compiler root must not absorb schema declaration parsing again'
fi
if grep -q '^use super::\*;' crates/compiler/src/schema_declarations.rs; then
  fail 'schema declaration module must keep explicit dependencies'
fi
max_lines crates/compiler/src/lib.rs 900
max_lines crates/compiler/src/schema_declarations.rs 450
printf '%s\n' 'R16 schema declaration responsibility extraction verification passed'

# R17: keep session storage and lifecycle policy out of the auth facade.
test -f crates/auth/src/session.rs || fail 'missing auth session module'
grep -q '^mod session;$' crates/auth/src/lib.rs || fail 'auth facade must own session module explicitly'
grep -q '^pub use session::{RedisSessionStore, SessionBackend, SessionFlash, SessionSnapshot, SessionStore};$' crates/auth/src/lib.rs || fail 'auth facade must preserve the public session API through explicit re-exports'
if grep -q '^pub struct SessionFlash\|^pub struct SessionSnapshot\|^pub struct SessionStore\|^pub struct RedisSessionStore\|^pub enum SessionBackend' crates/auth/src/lib.rs; then
  fail 'auth facade must not absorb session storage/lifecycle types again'
fi
if grep -q '^use super::\*;' crates/auth/src/session.rs; then
  fail 'auth session module must keep explicit dependencies'
fi
max_lines crates/auth/src/lib.rs 950
max_lines crates/auth/src/session.rs 460
printf '%s\n' 'R17 auth session responsibility extraction verification passed'

# R18: keep local-user persistence and credential policy out of the auth facade.
test -f crates/auth/src/local_user.rs || fail 'missing local-user authentication module'
grep -q '^mod local_user;$' crates/auth/src/lib.rs || fail 'auth facade must own local_user module explicitly'
grep -q '^pub use local_user::{LocalUserAuth, LocalUserStore};$' crates/auth/src/lib.rs || fail 'auth facade must preserve the public local-user API through explicit re-exports'
if grep -q '^pub struct LocalUserAuth\|^pub struct LocalUserStore\|^fn canonical_local_username\|^fn validate_password\|^fn hash_password\|^fn recovery_hash' crates/auth/src/lib.rs; then
  fail 'auth facade must not absorb local-user persistence or credential policy again'
fi
if grep -q '^use super::\*;' crates/auth/src/local_user.rs; then
  fail 'local-user authentication module must keep explicit dependencies'
fi
max_lines crates/auth/src/lib.rs 380
max_lines crates/auth/src/local_user.rs 620
printf '%s\n' 'R18 local-user authentication responsibility extraction verification passed'

# R20: keep HTTP response/presentation concerns out of the server entrypoint.
test -f crates/server/src/presentation.rs || fail 'missing server presentation module'
grep -q '^mod presentation;$' crates/server/src/main.rs || fail 'server main must own presentation explicitly'
grep -Eq '^use presentation::(\{|endpoint_error;)' crates/server/src/main.rs || fail 'server main must import presentation helpers explicitly'
if grep -q '^fn app_error_response\|^fn read_error_response\|^fn render_form_failure\|^fn app_response_to_response\|^fn conflict_response\|^fn endpoint_error\|^fn accepts_media' crates/server/src/main.rs; then
  fail 'server main must not absorb HTTP presentation/response mapping again'
fi
if grep -q '^use super::\*;' crates/server/src/presentation.rs; then
  fail 'presentation module must keep explicit dependencies'
fi
max_lines crates/server/src/main.rs 1250
max_lines crates/server/src/presentation.rs 420
printf '%s\n' 'R20 response/presentation responsibility extraction verification passed'


# R21: keep authentication HTTP/session-cookie transport out of the server entrypoint.
test -f crates/server/src/auth_http.rs || fail 'missing server authentication HTTP module'
grep -q '^mod auth_http;$' crates/server/src/main.rs || fail 'server main must own auth_http explicitly'
grep -q '^use crate::auth_http::{parse_cookie, session_cookie_name};$' crates/server/src/request_pipeline.rs || fail 'request pipeline must import the auth HTTP/session-cookie helpers it uses explicitly'
if grep -q '^async fn auth_login\|^async fn auth_logout\|^fn audit_auth_activity\|^fn session_cookie_name\|^fn session_cookie\|^fn parse_cookie' crates/server/src/main.rs; then
  fail 'server main must not absorb authentication HTTP/session-cookie transport again'
fi
if grep -q '^use super::\*;' crates/server/src/auth_http.rs; then
  fail 'authentication HTTP module must keep explicit dependencies'
fi
max_lines crates/server/src/main.rs 950
max_lines crates/server/src/auth_http.rs 380
printf '%s\n' 'R21 authentication HTTP responsibility extraction verification passed'

# R22: keep process lifecycle and operational endpoints out of the server entrypoint.
test -f crates/server/src/operations.rs || fail 'missing server operations module'
grep -q '^mod operations;$' crates/server/src/main.rs || fail 'server main must own operations explicitly'
grep -q '^use operations::install_panic_logging_hook;$' crates/server/src/main.rs || fail 'server main must import only the operations helper it uses explicitly'
grep -q '^use crate::operations::serve_health_endpoint;$' crates/server/src/request_pipeline.rs || fail 'request pipeline must import serve_health_endpoint explicitly'
if grep -q '^fn install_panic_logging_hook\|^async fn shutdown_signal\|^async fn serve_health_endpoint\|^async fn run_http_redirect_listener\|^fn safe_redirect_location\|^async fn run_metrics_listener' crates/server/src/main.rs; then
  fail 'server main must not absorb lifecycle/operational endpoint behavior again'
fi
if grep -q '^use super::\*;' crates/server/src/operations.rs; then
  fail 'operations module must keep explicit dependencies'
fi
max_lines crates/server/src/main.rs 720
max_lines crates/server/src/operations.rs 280
printf '%s\n' 'R22 lifecycle/operations responsibility extraction verification passed'


# R23: keep request input adaptation out of the server entrypoint.
test -f crates/server/src/request_input.rs || fail 'missing server request input module'
grep -q '^mod request_input;$' crates/server/src/main.rs || fail 'server main must own request_input explicitly'
if grep -q '^use request_input::\*;' crates/server/src/main.rs; then fail 'server main must not wildcard-import request input helpers'; fi
if grep -q '^async fn build_upload_runtime_value\|^fn media_type_is\|^struct StrictJsonObjectVisitor\|^fn decode_json_object_limited' crates/server/src/main.rs; then
  fail 'server main must not absorb request input adaptation again'
fi
if grep -q '^use super::\*;' crates/server/src/request_input.rs; then
  fail 'request input module must keep explicit dependencies'
fi
max_lines crates/server/src/main.rs 600
max_lines crates/server/src/request_input.rs 160
printf '%s\n' 'R23 request input responsibility extraction verification passed'

# R24: orchestration boundaries must keep typed errors instead of dynamic error erasure.
test -f crates/server/src/server_errors/orchestration.rs || fail 'missing typed server orchestration errors'
grep -q '^pub(crate) enum StartupError' crates/server/src/server_errors/orchestration.rs || fail 'missing typed StartupError'
grep -q '^pub(crate) enum ConnectionError' crates/server/src/server_errors/orchestration.rs || fail 'missing typed ConnectionError'
grep -q '^impl From<auth::AuthError> for ConnectionError' crates/server/src/server_errors/orchestration.rs || fail 'ConnectionError must preserve authentication failures from session resolution'
grep -q '^enum CliError' crates/cli/src/main.rs || fail 'missing typed CLI orchestration error'
if grep -R --include='*.rs' -E 'Box<dyn (std::error::Error|Error)>' crates >/dev/null 2>&1; then
  fail 'production error boundaries must not erase errors behind Box<dyn Error>'
fi
printf '%s\n' 'R24 typed orchestration error verification passed'


# R25: data crate is a stable facade over responsibility-focused internal modules.
for file in database.rs error.rs redis_store.rs sql.rs types.rs; do
  test -f "crates/data/src/$file" || fail "missing data module crates/data/src/$file"
done
grep -q '^mod database;$' crates/data/src/lib.rs || fail 'data facade must own database module'
grep -q '^mod error;$' crates/data/src/lib.rs || fail 'data facade must own error module'
grep -q '^mod redis_store;$' crates/data/src/lib.rs || fail 'data facade must own redis_store module'
grep -q '^mod sql;$' crates/data/src/lib.rs || fail 'data facade must own sql module'
grep -q '^mod types;$' crates/data/src/lib.rs || fail 'data facade must own types module'
grep -q '^pub use database::{Database, DbTransaction};$' crates/data/src/lib.rs || fail 'data facade must preserve database API re-exports'
grep -q '^pub use error::DataError;$' crates/data/src/lib.rs || fail 'data facade must preserve DataError re-export'
grep -q '^pub use redis_store::{RedisConfig, RedisStore};$' crates/data/src/lib.rs || fail 'data facade must preserve Redis API re-exports'
grep -q '^pub use sql::{BindSet, PreparedSql};$' crates/data/src/lib.rs || fail 'data facade must preserve SQL API re-exports'
if grep -R '^use super::\*;' crates/data/src --include='*.rs' | grep -v 'src/lib.rs:' >/dev/null; then
  fail 'data production modules must keep explicit dependencies'
fi
max_lines crates/data/src/lib.rs 120
max_lines crates/data/src/database.rs 320
max_lines crates/data/src/sql.rs 360
max_lines crates/data/src/redis_store.rs 260
printf '%s\n' 'R25 data responsibility extraction verification passed'


# R26: storage crate is a stable facade over filesystem, upload, and image responsibilities.
for file in filesystem.rs upload.rs image.rs; do
  test -f "crates/storage/src/$file" || fail "missing storage module crates/storage/src/$file"
done
grep -q '^mod filesystem;$' crates/storage/src/lib.rs || fail 'storage facade must own filesystem module'
grep -q '^mod upload;$' crates/storage/src/lib.rs || fail 'storage facade must own upload module'
grep -q '^mod image;$' crates/storage/src/lib.rs || fail 'storage facade must own image module'
grep -q '^pub use filesystem::{AppFs, BoundedFile, FsError, FsLimits, FsMode, validate_relative_path};$' crates/storage/src/lib.rs || fail 'storage facade must preserve filesystem API re-exports'
grep -q '^pub use image::{ImageError, ImageInfo, inspect_image};$' crates/storage/src/lib.rs || fail 'storage facade must preserve image API re-exports'
grep -q '^pub use upload::{UploadError, UploadResult, multipart_boundary, store_single_multipart_file};$' crates/storage/src/lib.rs || fail 'storage facade must preserve upload API re-exports'
if grep -R --include='*.rs' -n '^ *use super::\*;' crates/storage/src/filesystem.rs crates/storage/src/upload.rs crates/storage/src/image.rs >/dev/null 2>&1; then
  fail 'storage responsibility modules must keep dependencies explicit'
fi
max_lines crates/storage/src/lib.rs 80
max_lines crates/storage/src/filesystem.rs 450
max_lines crates/storage/src/upload.rs 240
max_lines crates/storage/src/image.rs 180
printf '%s\n' 'R26 storage responsibility extraction verification passed'


# R27: integrations crate is a stable facade over egress policy, secrets, and HTTPS transport.
for file in egress.rs error.rs https_client.rs secrets.rs; do
  test -f "crates/integrations/src/$file" || fail "missing integrations module crates/integrations/src/$file"
done
grep -q '^mod egress;$' crates/integrations/src/lib.rs || fail 'integrations facade must own egress module'
grep -q '^mod error;$' crates/integrations/src/lib.rs || fail 'integrations facade must own error module'
grep -q '^mod https_client;$' crates/integrations/src/lib.rs || fail 'integrations facade must own https_client module'
grep -q '^mod secrets;$' crates/integrations/src/lib.rs || fail 'integrations facade must own secrets module'
grep -q '^pub use egress::{EgressConfig, EgressPolicy, TargetConfig};$' crates/integrations/src/lib.rs || fail 'integrations facade must preserve egress API re-exports'
grep -q '^pub use error::IntegrationError;$' crates/integrations/src/lib.rs || fail 'integrations facade must preserve IntegrationError re-export'
grep -q '^pub use http_response::{HttpsResponse, StatusResponse};$' crates/integrations/src/lib.rs || fail 'integrations facade must preserve bounded HTTPS response API re-exports'
grep -q '^pub use https_client::OutboundHttpsClient;$' crates/integrations/src/lib.rs || fail 'integrations facade must preserve HTTPS client re-export'
grep -q '^pub use secrets::{SecretString, SecretsStore};$' crates/integrations/src/lib.rs || fail 'integrations facade must preserve secrets API re-exports'
if grep -R --include='*.rs' -n '^ *use super::\*;' crates/integrations/src/egress.rs crates/integrations/src/secrets.rs crates/integrations/src/https_client.rs crates/integrations/src/http_response.rs crates/integrations/src/error.rs >/dev/null 2>&1; then
  fail 'integrations responsibility modules must keep dependencies explicit'
fi
max_lines crates/integrations/src/lib.rs 80
max_lines crates/integrations/src/egress.rs 300
max_lines crates/integrations/src/secrets.rs 160
max_lines crates/integrations/src/https_client.rs 340
max_lines crates/integrations/src/error.rs 80
printf '%s\n' 'R27 integrations responsibility extraction verification passed'

# R28: observability crate is a stable facade over events, metrics, logging, and error responsibilities.
for file in error.rs events.rs logging.rs metrics.rs; do
  test -f "crates/observability/src/$file" || fail "missing observability module crates/observability/src/$file"
done
grep -q '^mod error;$' crates/observability/src/lib.rs || fail 'observability facade must own error module'
grep -q '^mod events;$' crates/observability/src/lib.rs || fail 'observability facade must own events module'
grep -q '^mod logging;$' crates/observability/src/lib.rs || fail 'observability facade must own logging module'
grep -q '^mod metrics;$' crates/observability/src/lib.rs || fail 'observability facade must own metrics module'
grep -q '^pub use error::ObsError;$' crates/observability/src/lib.rs || fail 'observability facade must preserve ObsError re-export'
grep -q '^pub use events::{' crates/observability/src/lib.rs || fail 'observability facade must preserve event API re-exports'
for symbol in ActivityEvent AuditEvent RequestLog SecurityEvent SystemEvent json_line new_request_id utc_timestamp; do
  grep -q "$symbol" crates/observability/src/lib.rs || fail "observability facade missing event re-export: $symbol"
done
grep -q '^pub use logging::{' crates/observability/src/lib.rs || fail 'observability facade must preserve logging API re-exports'
for symbol in LogConfig LogManager access_log audit_log flush_logs init_logging reopen_logs security_event server_event server_log; do
  grep -q "$symbol" crates/observability/src/lib.rs || fail "observability facade missing logging re-export: $symbol"
done
grep -q '^pub use metrics::{ConnectionGuard, Metrics, RequestTimer};$' crates/observability/src/lib.rs || fail 'observability facade must preserve metrics API re-exports'
if grep -R --include='*.rs' -n '^ *use super::\*;' crates/observability/src/error.rs crates/observability/src/events.rs crates/observability/src/logging.rs crates/observability/src/metrics.rs >/dev/null 2>&1; then
  fail 'observability responsibility modules must keep production dependencies explicit'
fi
max_lines crates/observability/src/lib.rs 80
max_lines crates/observability/src/events.rs 180
max_lines crates/observability/src/metrics.rs 360
max_lines crates/observability/src/logging.rs 320
max_lines crates/observability/src/error.rs 80
printf '%s\n' 'R28 observability responsibility extraction verification passed'

# R29: the data::migrations module is a stable facade over source, history, database, locking, and service responsibilities.
for file in database.rs error.rs history.rs locking.rs service.rs source.rs types.rs; do
  test -f "crates/data/src/migrations/$file" || fail "missing migrations module crates/data/src/migrations/$file"
done
grep -q '^mod database;$' crates/data/src/migrations/mod.rs || fail 'migrations facade must own database module'
grep -q '^mod error;$' crates/data/src/migrations/mod.rs || fail 'migrations facade must own error module'
grep -q '^mod history;$' crates/data/src/migrations/mod.rs || fail 'migrations facade must own history module'
grep -q '^mod locking;$' crates/data/src/migrations/mod.rs || fail 'migrations facade must own locking module'
grep -q '^mod service;$' crates/data/src/migrations/mod.rs || fail 'migrations facade must own service module'
grep -q '^mod source;$' crates/data/src/migrations/mod.rs || fail 'migrations facade must own source module'
grep -q '^mod types;$' crates/data/src/migrations/mod.rs || fail 'migrations facade must own types module'
grep -q '^pub use error::MigrationError;$' crates/data/src/migrations/mod.rs || fail 'migrations facade must preserve MigrationError re-export'
grep -q '^pub use service::{apply, status, verify};$' crates/data/src/migrations/mod.rs || fail 'migrations facade must preserve service API re-exports'
grep -q '^pub use source::{load_migrations, split_sql_statements};$' crates/data/src/migrations/mod.rs || fail 'migrations facade must preserve source API re-exports'
grep -q '^pub use types::{AppliedMigration, Migration, MigrationState, MigrationStatus};$' crates/data/src/migrations/mod.rs || fail 'migrations facade must preserve migration type re-exports'
if grep -R --include='*.rs' -n '^ *use super::\*;' crates/data/src/migrations >/dev/null 2>&1; then
  fail 'migrations modules must keep dependencies explicit'
fi
max_lines crates/data/src/migrations/mod.rs 80
max_lines crates/data/src/migrations/source.rs 240
max_lines crates/data/src/migrations/service.rs 190
max_lines crates/data/src/migrations/database.rs 150
max_lines crates/data/src/migrations/locking.rs 120
max_lines crates/data/src/migrations/history.rs 80
printf '%s\n' 'R29 migrations responsibility extraction verification passed'


# R30: language-core begins decomposing foundational web/value types behind a stable facade.
for file in values.rs web_types.rs; do
  test -f "crates/language-core/src/$file" || fail "missing language-core module crates/language-core/src/$file"
done
grep -q '^mod values;$' crates/language-core/src/lib.rs || fail 'language-core facade must own values module'
grep -q '^mod web_types;$' crates/language-core/src/lib.rs || fail 'language-core facade must own web_types module'
compact_contains crates/language-core/src/lib.rs 'pubusevalues::{F32Value,FunctionParam,ImageRef,PageParam,Value,ValueType};' || fail 'language-core facade must preserve value API re-exports'
compact_contains crates/language-core/src/lib.rs 'pubuseweb_types::{FlashKind,FlashMessage,Html,HttpMethod,LocalUrl,Redirect,RedirectStatus};' || fail 'language-core facade must preserve web type API re-exports'
if grep -R --include='*.rs' -n '^ *use super::\*;' crates/language-core/src/values.rs crates/language-core/src/web_types.rs >/dev/null 2>&1; then
  fail 'language-core foundation modules must keep dependencies explicit'
fi
max_lines crates/language-core/src/lib.rs 720
max_lines crates/language-core/src/values.rs 280
max_lines crates/language-core/src/web_types.rs 140
printf '%s\n' 'R30 language-core foundation responsibility extraction verification passed'

# R31: language-core AST/expression/statement layer lives behind the stable facade.
test -f crates/language-core/src/ast.rs || fail 'missing language-core AST module'
grep -q '^mod ast;$' crates/language-core/src/lib.rs || fail 'language-core facade must own ast module'
compact_contains crates/language-core/src/lib.rs 'pubuseast::{ActionBody,ActionFunction,ActionMatchArm,ActionStatement,BinaryOp,BusinessAudit,ComponentFunction,ComputeStatement,Expr,HtmlAttrKind,HtmlPart,HtmlTemplate,InherentMethod,InlineHint,LayoutFunction,OutboundCall,OutboundMethod,PageBody,PageFunction,PageMatchArm,PureFunction,PureFunctionParam,PureParamType,PureSumPredicate,QueryCall,ResourceUse,RouteCall,SourceLocation,Statement,TemplateParam,TemplateParamType,TxStatement,TypedJsonField};' || fail 'language-core facade must preserve AST API re-exports including inherent methods and verified pure functions'
test -f crates/language-core/src/pure_types.rs || fail 'missing language-core pure type module'
grep -q '^mod pure_types;$' crates/language-core/src/lib.rs || fail 'language-core facade must own pure_types module'
grep -q '^pub use pure_types::{PureReturnType, PureValueType};$' crates/language-core/src/lib.rs || fail 'language-core facade must preserve pure type API re-exports'
test -f crates/language-core/src/builtin.rs || fail 'missing language-core builtin domain module'
grep -q '^mod builtin;$' crates/language-core/src/lib.rs || fail 'language-core facade must own builtin module'
grep -q '^pub use builtin::{BuiltinExecutionKind, BuiltinFunction, BuiltinMetadata};$' crates/language-core/src/lib.rs || fail 'language-core facade must preserve builtin API re-exports'
test -f crates/language-core/src/authorization.rs || fail 'missing language-core authorization domain module'
grep -q '^mod authorization;$' crates/language-core/src/lib.rs || fail 'language-core facade must own authorization module'
grep -q '^pub use authorization::{AuthorizationMode, ObjectAuthorization};$' crates/language-core/src/lib.rs || fail 'language-core facade must preserve authorization API re-exports'
if grep -R --include='*.rs' -n '^ *use super::\*;' crates/language-core/src/ast.rs >/dev/null 2>&1; then
  fail 'language-core AST module must keep dependencies explicit'
fi
max_lines crates/language-core/src/lib.rs 340
max_lines crates/language-core/src/ast.rs 450
printf '%s\n' 'R31 language-core AST responsibility extraction verification passed'

# R32: language-core root is a thin facade over schema/query/routing/program/config/error responsibilities.
for file in config.rs error.rs program.rs query.rs routing.rs schema.rs; do
  test -f "crates/language-core/src/$file" || fail "missing language-core module crates/language-core/src/$file"
done
for module in config error program query routing schema; do
  grep -q "^mod $module;$" crates/language-core/src/lib.rs || fail "language-core facade must own $module module"
done
grep -q '^pub use config::ServerConfig;$' crates/language-core/src/lib.rs || fail 'language-core facade must preserve ServerConfig re-export'
grep -q '^pub use error::AppError;$' crates/language-core/src/lib.rs || fail 'language-core facade must preserve AppError re-export'
grep -q '^pub use program::Program;$' crates/language-core/src/lib.rs || fail 'language-core facade must preserve Program re-export'
compact_contains crates/language-core/src/lib.rs 'pubusequery::{CredentialLifecycleMode,CredentialLifecycleTarget,MutationTarget,QueryCapability,QueryFunction,QueryReturn,TenantScopeTarget};' || fail 'language-core facade must preserve query API re-exports'
grep -q '^pub use routing::{PublicCachePolicy, Route, RouteAuth, RouteSegment, UploadField};$' crates/language-core/src/lib.rs || fail 'language-core facade must preserve routing API re-exports'
compact_contains crates/language-core/src/lib.rs 'pubuseschema::{EnumDef,FormFailure,FormField,FormFieldIssue,FormSchema,JsonSchema,Model,ValidationKind,ValidationRule};' || fail 'language-core facade must preserve schema API re-exports'
if grep -R --include='*.rs' -n '^ *use super::\*;' crates/language-core/src/config.rs crates/language-core/src/error.rs crates/language-core/src/program.rs crates/language-core/src/query.rs crates/language-core/src/routing.rs crates/language-core/src/schema.rs >/dev/null 2>&1; then
  fail 'language-core R32 responsibility modules must keep dependencies explicit'
fi
max_lines crates/language-core/src/lib.rs 80
max_lines crates/language-core/src/schema.rs 100
max_lines crates/language-core/src/query.rs 80
max_lines crates/language-core/src/routing.rs 100
max_lines crates/language-core/src/program.rs 100
max_lines crates/language-core/src/config.rs 80
max_lines crates/language-core/src/error.rs 80
printf '%s\n' 'R32 language-core program/schema responsibility extraction verification passed'


# R33: compiler expression parsing/type analysis lives behind a dedicated responsibility boundary.
test -f crates/compiler/src/expression.rs || fail 'missing compiler expression module'
grep -q '^mod expression;$' crates/compiler/src/lib.rs || fail 'compiler root must own expression module'
if grep -q '^pub(crate) use expression::' crates/compiler/src/lib.rs; then
  fail 'compiler root must not re-export expression internals after R35'
fi
if grep -q '^struct ExprParser\|^pub(crate) enum ExprToken\|^fn parse_expr\|^fn validate_expr\|^fn infer_static_expr_type\|^fn infer_expr_type' crates/compiler/src/lib.rs; then
  fail 'compiler root must not absorb expression parsing or type-analysis implementation again'
fi
grep -q '^mod expression_token;$' crates/compiler/src/lib.rs || fail 'compiler root must own expression token module'
grep -q '^use crate::expression_token::ExprToken;$' crates/compiler/src/lexer.rs || fail 'lexer must depend explicitly on the expression token owner'
if grep -q '^ *use super::\*;' crates/compiler/src/expression.rs; then
  fail 'compiler expression module must keep dependencies explicit'
fi
max_lines crates/compiler/src/lib.rs 450
max_lines crates/compiler/src/expression.rs 500
printf '%s\n' 'R33 compiler expression responsibility extraction verification passed'

# R34: compiler source and SQL syntax helpers live behind dedicated utility boundaries.
for file in source_syntax.rs sql_syntax.rs; do
  test -f "crates/compiler/src/$file" || fail "missing compiler utility module crates/compiler/src/$file"
done
grep -q '^mod source_syntax;$' crates/compiler/src/lib.rs || fail 'compiler root must own source_syntax module'
grep -q '^mod sql_syntax;$' crates/compiler/src/lib.rs || fail 'compiler root must own sql_syntax module'
if grep -q '^pub(crate) use source_syntax::\|^pub(crate) use sql_syntax::' crates/compiler/src/lib.rs; then
  fail 'compiler root must not re-export syntax internals after R35'
fi
if grep -q '^fn first_sql_keyword\|^fn scan_bind_names\|^fn line_number\|^fn function_bounds\|^fn read_ident\|^fn is_identifier\|^fn matching_brace\|^fn matching_paren\|^fn split_top_level\|^fn find_statement_end\|^fn skip_ws_and_comments\|^fn consume_return_tail\|^fn preview' crates/compiler/src/lib.rs; then
  fail 'compiler root must not absorb source/SQL utility implementations again'
fi
if grep -R --include='*.rs' -n '^ *use super::\*;' crates/compiler/src/source_syntax.rs crates/compiler/src/sql_syntax.rs >/dev/null 2>&1; then
  fail 'compiler utility modules must keep dependencies explicit'
fi
max_lines crates/compiler/src/lib.rs 200
max_lines crates/compiler/src/source_syntax.rs 230
max_lines crates/compiler/src/sql_syntax.rs 110
printf '%s\n' 'R34 compiler utility responsibility extraction verification passed'


# R35: production compiler modules depend on owning modules instead of the crate root wildcard hub.
for file in builtin_types.rs page_statements.rs routes.rs handler_parser.rs action_statements.rs query_parser.rs statement_helpers.rs control_flow.rs template_declarations.rs html_template.rs; do
  if grep -q '^use super::\*;' "crates/compiler/src/$file"; then
    fail "compiler production module $file must keep dependencies explicit"
  fi
done
for file in arrays.rs dicts.rs regex_types.rs domain_objects.rs schema_declarations.rs source_loader.rs; do
  if grep -q '^use crate::{.*\(infer_expr_type\|parse_expr\|validate_expr\|is_identifier\|matching_brace\|resolve_value_type\|scan_bind_names\)' "crates/compiler/src/$file"; then
    fail "compiler module $file must import helpers from their owning modules"
  fi
done
test -f crates/compiler/src/domain_symbols.rs || fail 'missing compiler domain_symbols module'
test -f crates/compiler/src/type_resolution.rs || fail 'missing compiler type_resolution module'
grep -q '^mod domain_symbols;$' crates/compiler/src/lib.rs || fail 'compiler root must own domain_symbols module'
grep -q '^mod type_resolution;$' crates/compiler/src/lib.rs || fail 'compiler root must own type_resolution module'
if grep -q '^fn internal_domain_symbol\|^fn display_domain_symbol\|^fn resolve_value_type\|^fn source_error' crates/compiler/src/lib.rs; then
  fail 'compiler root must not absorb symbol/type/source helper implementations again'
fi
max_lines crates/compiler/src/lib.rs 190
max_lines crates/compiler/src/domain_symbols.rs 60
max_lines crates/compiler/src/type_resolution.rs 50
printf '%s\n' 'R35 compiler explicit dependency verification passed'

# R36: runtime request orchestration delegates statement interpretation to a dedicated boundary.
test -f crates/runtime/src/response.rs || fail 'missing runtime response module'
grep -q '^mod response;$' crates/runtime/src/lib.rs || fail 'runtime facade must own response explicitly'
if grep -q '^ *use super::\*;' crates/runtime/src/response.rs; then
  fail 'runtime statement/response modules must keep dependencies explicit'
fi
max_lines crates/runtime/src/response.rs 40
printf '%s\n' 'R36 runtime statement execution boundary verification passed'

# R36.2: compiler production modules retain explicit owning-module imports after R35 cleanup.
grep -q '^use crate::{action_statements, control_flow, declarations, page_statements};$' crates/compiler/src/handler_parser.rs || fail 'handler parser must import compiler collaborators explicitly'
grep -q '^use crate::{arrays, control_flow, dicts, sum_match};$' crates/compiler/src/page_statements.rs || fail 'page statements must import execution/parser collaborators explicitly'
grep -q '^use crate::{arrays, control_flow, dicts, sum_match};$' crates/compiler/src/action_statements.rs || fail 'action statements must import execution/parser collaborators explicitly'
grep -q '^use crate::{arrays, dicts};$' crates/compiler/src/control_flow.rs || fail 'control flow must import collection statement helpers explicitly'
grep -q '^use crate::regex_types;$' crates/compiler/src/builtin_types.rs || fail 'builtin type inference must import regex type owner explicitly'
! grep -q '^use crate::schema_declarations;$' crates/compiler/src/routes.rs || fail 'routes must not depend on schema declaration parser internals'
compact_contains crates/compiler/src/routes.rs 'usecrate::cache_safety::{action_has_business_audit,action_has_object_auth,validate_public_cache_statements,};' || fail 'routes must import cache safety helpers explicitly'
grep -q '^use std::collections::HashMap;$' crates/compiler/src/routes.rs || fail 'routes must own its HashMap dependency explicitly'
printf '%s\n' 'R36.2 compiler explicit import build-fix verification passed'


# R36.3: nested compiler control-flow tests import the public compile entrypoint explicitly.
grep -A4 '^mod tests {' crates/compiler/src/control_flow.rs | grep -q '^    use crate::compile_source;$' || fail 'control_flow tests must import compile_source explicitly'
grep -A4 '^mod if_tests {' crates/compiler/src/control_flow.rs | grep -q '^    use crate::compile_source;$' || fail 'control_flow if_tests must import compile_source explicitly'
if grep -q '^pub(crate) use expression::\|^pub(crate) use source_syntax::\|^pub(crate) use sql_syntax::' crates/compiler/src/lib.rs; then
  fail 'compiler root must not expose production helper compatibility re-exports'
fi
printf '%s\n' 'R36.3 compiler test-scope import verification passed'

# R37: compiler tests are organized behind a dedicated test facade, not the crate root.
test -f crates/compiler/src/tests/core_compile_tests.rs || fail 'missing compiler core compile tests module'
test -f crates/compiler/src/tests/presentation_compile_tests.rs || fail 'missing compiler presentation compile tests module'
test -f crates/compiler/src/tests/domain_compile_tests.rs || fail 'missing compiler domain compile tests module'
test -f crates/compiler/src/tests/module_namespace_compile_tests.rs || fail 'missing compiler module namespace compile tests module'
test -f crates/compiler/src/tests/data_contract_compile_tests.rs || fail 'missing compiler data-contract compile tests module'
test -f crates/compiler/src/tests/web_flow_compile_tests.rs || fail 'missing compiler web-flow compile tests module'
if awk 'prev == "#[cfg(test)]" && $0 ~ /^use / { found=1 } { prev=$0 } END { exit(found ? 0 : 1) }' crates/compiler/src/lib.rs; then
  fail 'compiler crate root must not own the legacy test prelude imports'
fi
grep -q '^mod core_compile_tests;$' crates/compiler/src/tests.rs || fail 'compiler test facade must include core compile tests'
grep -q '^mod presentation_compile_tests;$' crates/compiler/src/tests.rs || fail 'compiler test facade must include presentation compile tests'
grep -q '^mod domain_compile_tests;$' crates/compiler/src/tests.rs || fail 'compiler test facade must include domain compile tests'
grep -q '^mod module_namespace_compile_tests;$' crates/compiler/src/tests.rs || fail 'compiler test facade must include module namespace compile tests'
grep -q '^mod data_contract_compile_tests;$' crates/compiler/src/tests.rs || fail 'compiler test facade must include data-contract compile tests'
grep -q '^mod web_flow_compile_tests;$' crates/compiler/src/tests.rs || fail 'compiler test facade must include web-flow compile tests'
max_lines crates/compiler/src/tests.rs 60
max_lines crates/compiler/src/tests/core_compile_tests.rs 360
max_lines crates/compiler/src/tests/presentation_compile_tests.rs 220
max_lines crates/compiler/src/tests/domain_compile_tests.rs 360
max_lines crates/compiler/src/tests/module_namespace_compile_tests.rs 240
max_lines crates/compiler/src/tests/data_contract_compile_tests.rs 420
max_lines crates/compiler/src/tests/web_flow_compile_tests.rs 160
printf '%s\n' 'R37 compiler test architecture verification passed'

# R38: legacy runtime interpreter tests were removed with the native-only cutover.
printf '%s\n' 'R38 native-only runtime test cleanup verification passed'

# R39: server tests are organized behind a dedicated test facade by responsibility.
for file in http_security_tests.rs config_lifecycle_tests.rs rate_observability_tests.rs response_boundary_tests.rs; do
  test -f "crates/server/src/main_tests/$file" || fail "missing server test responsibility module crates/server/src/main_tests/$file"
done
grep -q '^mod http_security_tests;$' crates/server/src/main_tests.rs || fail 'server test facade must include HTTP/security tests'
grep -q '^mod config_lifecycle_tests;$' crates/server/src/main_tests.rs || fail 'server test facade must include config/lifecycle tests'
grep -q '^mod rate_observability_tests;$' crates/server/src/main_tests.rs || fail 'server test facade must include rate/observability tests'
grep -q '^mod response_boundary_tests;$' crates/server/src/main_tests.rs || fail 'server test facade must include response/boundary tests'
max_lines crates/server/src/main_tests.rs 20
max_lines crates/server/src/main_tests/http_security_tests.rs 560
max_lines crates/server/src/main_tests/config_lifecycle_tests.rs 240
max_lines crates/server/src/main_tests/rate_observability_tests.rs 200
max_lines crates/server/src/main_tests/response_boundary_tests.rs 100
printf '%s\n' 'R39 server test architecture verification passed'

# R39.1: test-layout splits remain syntactically valid and path-stable.
for file in core_compile_tests.rs presentation_compile_tests.rs domain_compile_tests.rs module_namespace_compile_tests.rs data_contract_compile_tests.rs web_flow_compile_tests.rs; do
  if tail -n 1 "crates/compiler/src/tests/$file" | grep -q '^#\[cfg(test)\]$'; then
    fail "compiler responsibility test module must not end with a dangling #[cfg(test)] attribute: $file"
  fi
done
grep -q '^#!\[allow(unused_imports)\]$' crates/compiler/src/tests.rs || fail 'compiler test facade must document intentional shared-prelude imports'
grep -q 'PageFunction, Program,' crates/compiler/src/tests.rs || fail 'compiler test facade must expose Program to child test modules'
grep -Fq '../../../../config/server.toml.sample' crates/server/src/main_tests/config_lifecycle_tests.rs || fail 'server config test sample path must account for main_tests directory'
grep -Fq '../../../../config/server-multidomain.toml.sample' crates/server/src/main_tests/config_lifecycle_tests.rs || fail 'server multidomain sample path must account for main_tests directory'
grep -Fq '../../../../config/domains/domain.toml.sample' crates/server/src/main_tests/config_lifecycle_tests.rs || fail 'server domain sample path must account for main_tests directory'
printf '%s\n' 'R39.1 test-layout build-fix verification passed'

# R39.2: responsibility compiler test files must inherit the shared tests-module prelude.
for f in \
  crates/compiler/src/tests/core_compile_tests.rs \
  crates/compiler/src/tests/presentation_compile_tests.rs \
  crates/compiler/src/tests/domain_compile_tests.rs \
  crates/compiler/src/tests/module_namespace_compile_tests.rs \
  crates/compiler/src/tests/data_contract_compile_tests.rs \
  crates/compiler/src/tests/web_flow_compile_tests.rs
do
  grep -q '^use super::\*;$' "$f" || fail "$f must import the shared compiler test prelude"
done
grep -q 'compile_file_with_dependencies' crates/compiler/src/tests.rs || fail 'compiler test prelude must expose compile_file_with_dependencies'
grep -q '^use std::path::PathBuf;$' crates/compiler/src/tests.rs || fail 'compiler test prelude must expose PathBuf'
printf '%s\n' 'R39.2 compiler shared test-prelude verification passed'


# R40: final workspace crate-boundary consolidation.
test ! -d crates/resource-limits || fail 'resource-limits must remain a server module, not a standalone micro-crate'
grep -q 'cargo test --locked -p velran-server resource_limits::tests' verify.sh || fail 'verify.sh must test resource limits through velran-server after crate consolidation'
if grep -q 'cargo test --locked -p resource-limits' verify.sh; then
  fail 'verify.sh must not reference removed resource-limits package'
fi
grep -q 'test -f crates/server/src/resource_limits.rs' verify.sh || fail 'verify.sh must check consolidated server resource-limits module'
grep -q '^mod resource_limits;$' crates/server/src/main.rs || fail 'server must own resource_limits module'
if grep -q 'resource-limits = { path = "../resource-limits" }' crates/server/Cargo.toml; then
  fail 'server must not depend on removed resource-limits crate'
fi
if grep -q '"crates/resource-limits"' Cargo.toml; then
  fail 'workspace must not restore resource-limits micro-crate'
fi
if grep -q 'resource-limits' Cargo.lock; then
  fail 'Cargo.lock must not retain removed resource-limits package/dependency'
fi
max_lines crates/server/src/resource_limits.rs 280
printf '%s\n' 'R40 crate-boundary consolidation verification passed'

# R41: server production child modules use explicit dependencies instead of parent wildcard imports.
for f in crates/server/src/*.rs; do
  case "$f" in
    crates/server/src/main.rs|crates/server/src/main_tests.rs) continue ;;
  esac
  if grep -q '^use super::\*;$' "$f"; then
    fail "$f must not depend on the server root through use super::*"
  fi
done
for f in \
  crates/server/src/backend_support.rs \
  crates/server/src/bootstrap_config.rs \
  crates/server/src/cli.rs \
  crates/server/src/cli_config_apply.rs \
  crates/server/src/connection.rs \
  crates/server/src/connection_dispatch.rs \
  crates/server/src/connection_finalize.rs \
  crates/server/src/http_dispatch.rs \
  crates/server/src/server_config_file.rs \
  crates/server/src/source_reload.rs \
  crates/server/src/startup.rs \
  crates/server/src/static_delivery.rs \
  crates/server/src/tls_support.rs
do
  test -f "$f" || fail "missing R41 server responsibility module: $f"
done
grep -q '^use crate::connection_dispatch;$' crates/server/src/connection.rs || fail 'connection must name its dispatch module dependency explicitly'
grep -q '^use crate::request_pipeline::{' crates/server/src/connection.rs || fail 'connection must name its request-pipeline dependency explicitly'
for symbol in AdmissionError admit_domain_request dispatch_early_request resolve_session select_domain; do
  sed -n '1,20p' crates/server/src/connection.rs | grep -qw "$symbol" \
    || fail "connection must import request-pipeline dependency ${symbol} explicitly"
done
grep -q '^use crate::backend_support::build_hosting_runtime;$' crates/server/src/startup.rs || fail 'startup must name hosting-runtime construction explicitly'
grep -q '^use crate::backend_support::{bind_application_listener, try_reload_hosting};$' crates/server/src/startup_transport.rs || fail 'startup transport must name listener/reload dependencies explicitly'
grep -q '^use crate::bootstrap_config::{json_log_escape, read_secret_file};$' crates/server/src/cli_config_apply.rs || fail 'CLI config application must name bootstrap helpers explicitly'

grep -q '^use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};$' crates/server/src/connection.rs || fail 'connection must import AsyncWriteExt for graceful shutdown'
grep -q '^use observability::{LogConfig, server_log};$' crates/server/src/cli_config_apply.rs || fail 'CLI config application must import server_log explicitly'
printf '%s\n' 'R41 server explicit dependency verification passed'

# R42: keep the server crate root production-focused and move test-only dependencies into main_tests.
for needle in \
  'RedisSessionStore' \
  'compile_file_with_dependencies' \
  'ExecutionLimits' \
  'AsyncReadExt' \
  'TlsAcceptor' \
  'build_upload_runtime_value' \
  'accepts_media'
do
  if grep -q "$needle" crates/server/src/main.rs; then
    fail "server main must not retain test-only/stale root import: $needle"
  fi
done
grep -q '^#!\[allow(unused_imports)\]$' crates/server/src/main_tests.rs || fail 'server test facade must own its intentional shared test prelude'
grep -q '^pub(super) use runtime::ExecutionLimits;$' crates/server/src/main_tests/test_support.rs || fail 'server test support must own ExecutionLimits test dependency'
grep -q '^pub(super) use storage::{FsLimits, FsMode};$' crates/server/src/main_tests/test_support.rs || fail 'server test support must own storage limit test dependencies'
grep -q '^    #\[cfg(not(target_os = "linux"))\]$' crates/server/src/resource_limits.rs || fail 'Linux-only resource-limit error variant must be cfg-gated on non-Linux builds'
printf '%s\n' 'R42 server root import cleanup verification passed'

# R42.1: server responsibility tests inherit the shared test prelude after root cleanup.
grep -q '^pub(super) use std::fs;$' crates/server/src/main_tests/test_support.rs || fail 'server test support must own std::fs for file-backed tests'
grep -q '^pub(super) use crate::bootstrap_config::{' crates/server/src/main_tests/test_support.rs || fail 'server test support must own bootstrap test helpers'
grep -q 'load_resource_profiles' crates/server/src/main_tests/test_support.rs || fail 'server test support must expose load_resource_profiles'
grep -q 'validate_route_rate_policies' crates/server/src/main_tests/test_support.rs || fail 'server test support must expose validate_route_rate_policies'
grep -q 'read_secret_file' crates/server/src/main_tests/test_support.rs || fail 'server test support must expose read_secret_file'
printf '%s\n' 'R42.1 server test-prelude verification passed'

# R43: server crate root must not act as a wildcard facade for child modules.
test -f crates/server/src/main_tests/test_support.rs || fail 'missing server shared test-support module'
max_lines crates/server/src/main_tests/test_support.rs 40
for pattern in \
  'use server_errors::\*;' \
  'use tls_support::\*;' \
  'use server_config_file::\*;' \
  'use http_io::\*;' \
  'use web_security::\*;' \
  'use static_delivery::\*;'
do
  if grep -Fq "$pattern" crates/server/src/main.rs; then
    fail "server root must not wildcard-import child module: $pattern"
  fi
done
grep -q '^use server_errors::{ClockError, ReservedPathError};$' crates/server/src/main.rs || fail 'server root must name runtime boundary errors explicitly'
grep -q '^use crate::server_config_file::{DomainCliConfig, NativeCliConfig, SourceReloadCliConfig};$' crates/server/src/startup_args.rs || fail 'startup args must name server config runtime types explicitly'
grep -q '^use http_io::Response;$' crates/server/src/main.rs || fail 'server root must name HTTP response boundary explicitly'
grep -q '^pub(super) use crate::http_io::{HttpReadError, HttpRequest, Response, parse_request_head};$' crates/server/src/main_tests/test_support.rs || fail 'server test support must own HTTP test imports explicitly'
grep -q '^pub(super) use crate::tls_support::{host_matches_public, validate_public_host};$' crates/server/src/main_tests/test_support.rs || fail 'server test support must own TLS helper imports explicitly'
printf '%s\n' 'R43 server root facade cleanup verification passed'


# R44: keep CLI parsing split by responsibility.
grep -q 'cli_scan::scan(&raw_args)' crates/server/src/cli.rs || fail 'R44 CLI orchestrator must delegate bootstrap flag scanning'
grep -q 'cli_overrides::apply(raw_args, loaded)' crates/server/src/cli.rs || fail 'R44 CLI orchestrator must delegate CLI overrides'
grep -q 'cli_finalize::finalize(applied, bootstrap)' crates/server/src/cli.rs || fail 'R44 CLI orchestrator must delegate final validation and modes'
if grep -q 'while let Some(arg) = args.next()' crates/server/src/cli.rs; then
  fail 'R44 CLI orchestrator must not absorb override parsing again'
fi
if grep -q 'load_rate_policies\|build_domain_configs\|print_effective_config' crates/server/src/cli.rs; then
  fail 'R44 CLI orchestrator must not absorb final validation/check-config responsibilities again'
fi
printf '%s\n' 'R44 CLI responsibility split verification passed'

# R45: named startup argument boundary.
max_lines crates/server/src/startup_args.rs 80
grep -q '^pub(super) struct StartupArgs' crates/server/src/startup_args.rs || fail 'missing named StartupArgs boundary'
grep -q 'Result<StartupArgs, CliParseError>' crates/server/src/cli.rs || fail 'CLI parser must return named StartupArgs'
grep -q 'Result<StartupArgs, CliParseError>' crates/server/src/cli_finalize.rs || fail 'CLI finalizer must construct named StartupArgs'
if grep -Rqs --include='*.rs' 'ParsedArgs' crates/server/src; then
  fail 'legacy ParsedArgs tuple must not return after R45'
fi
if grep -Eq 'parsed\.[0-9]+' crates/server/src/main.rs; then
  fail 'server main must not use tuple-indexed startup arguments'
fi

# R46: startup service preparation is a separate responsibility from listener orchestration.
test -f crates/server/src/startup_services.rs || fail 'missing server startup service-preparation module'
max_lines crates/server/src/startup.rs 360
max_lines crates/server/src/startup_services.rs 340
grep -q '^mod startup_services;$' crates/server/src/main.rs || fail 'server root must declare startup_services module'
grep -q 'startup_services::prepare' crates/server/src/startup.rs || fail 'startup orchestration must delegate service preparation'
grep -q '^pub(super) struct ServicePreparation' crates/server/src/startup_services.rs || fail 'startup service preparation must use a named input boundary'
grep -q '^pub(super) struct PreparedServices' crates/server/src/startup_services.rs || fail 'startup service preparation must return a named service bundle'
if grep -q 'Database::connect\|RedisStore::connect\|LocalUserStore::connect_sqlite' crates/server/src/startup.rs; then
  fail 'startup orchestrator must not own database/auth/cache connection setup after R46'
fi
printf '%s\n' 'R46 startup service preparation verification passed'

# R46.1: startup/reload and request pipeline must use owning modules directly.
grep -q '^use crate::resource_limits::ResourceLimitConfig;$' crates/server/src/startup_args.rs || fail 'startup args must import consolidated resource limits through crate ownership'
grep -q '^use crate::server_config_file::{DomainRuntime, HostingRuntime};$' crates/server/src/request_pipeline.rs || fail 'request pipeline must import domain runtimes from server_config_file'
grep -q '^use crate::tls_support::request_public_host;$' crates/server/src/request_pipeline.rs || fail 'request pipeline must import public-host parsing from tls_support'
grep -q '^use crate::http_io::HttpRequest;$' crates/server/src/request_pipeline.rs || fail 'request pipeline must import HttpRequest from http_io'
grep -q '^use crate::http_io::HttpRequest;$' crates/server/src/web_security.rs || fail 'web security must import HttpRequest from http_io'
if grep -Eq 'reloaded\.[0-9]+' crates/server/src/backend_support.rs; then
  fail 'hosting reload must use named StartupArgs fields, not tuple indices'
fi
grep -q '&reloaded.app' crates/server/src/backend_support.rs || fail 'hosting reload must use named app field'
grep -q '&reloaded.domains' crates/server/src/backend_support.rs || fail 'hosting reload must use named domains field'
printf '%s\n' 'R46.1 startup/request dependency build-fix verification passed'

# R46.2: server root must stay free of request/auth helpers owned by child modules,
# while the server test prelude explicitly owns LogConfig used by lifecycle tests.
if grep -Eq '^use auth_http::\{.*(parse_cookie|session_cookie_name)' crates/server/src/main.rs; then
  echo "R46.2 violation: main.rs must not import auth_http request helpers" >&2
  exit 1
fi
if grep -Eq '^use operations::\{.*serve_health_endpoint' crates/server/src/main.rs; then
  echo "R46.2 violation: main.rs must not import serve_health_endpoint" >&2
  exit 1
fi
grep -q '^pub(super) use observability::LogConfig;' crates/server/src/main_tests/test_support.rs || {
  echo "R46.2 violation: server test prelude must explicitly import LogConfig" >&2
  exit 1
}
# R47: transport/listener lifecycle is separate from startup validation and service preparation.
test -f crates/server/src/startup_transport.rs || fail 'missing server startup transport module'
max_lines crates/server/src/startup.rs 180
max_lines crates/server/src/startup_transport.rs 330
grep -q '^mod startup_transport;$' crates/server/src/main.rs || fail 'server root must declare startup_transport module'
grep -q '^pub(super) struct TransportRuntime' crates/server/src/startup_transport.rs || fail 'startup transport must use a named runtime boundary'
grep -q 'startup_transport::serve' crates/server/src/startup.rs || fail 'startup orchestrator must delegate listener lifecycle to startup_transport'
if grep -Eq 'bind_application_listener|JoinSet|shutdown_signal|run_metrics_listener|run_http_redirect_listener' crates/server/src/startup.rs; then
  fail 'startup orchestrator must not own listener/transport lifecycle after R47'
fi
grep -q 'bind_application_listener' crates/server/src/startup_transport.rs || fail 'startup transport must own application listener binding'
grep -q 'shutdown_signal' crates/server/src/startup_transport.rs || fail 'startup transport must own shutdown signal handling'
grep -q 'drain_connections' crates/server/src/startup_transport.rs || fail 'startup transport must own graceful connection draining'
printf '%s\n' 'R47 startup transport lifecycle verification passed'

# R48: modules are application-root-relative namespaces, not global textual includes.
test -f crates/compiler/src/module_namespace.rs || fail 'missing compiler module namespace helper'
test -f docs/21-modules-and-namespaces.md || fail 'missing canonical module namespace documentation'
test -f docs/hu/21-modules-slugs-project-layout.md || fail 'missing Hungarian module namespace documentation'
grep -q '^mod module_namespace;$' crates/compiler/src/lib.rs || fail 'compiler must declare module_namespace ownership module'
grep -q 'pub(crate) module_path: Vec<String>' crates/compiler/src/source_loader.rs || fail 'source units must carry their absolute module path'
grep -q 'pub(crate) fn namespace(&self)' crates/compiler/src/source_loader.rs || fail 'source units must expose their namespace'
grep -q 'candidate.push(segment)' crates/compiler/src/source_loader/tree.rs || fail 'module resolver must map namespace segments under application root'
grep -q 'candidate.set_extension("vrn")' crates/compiler/src/source_loader/tree.rs || fail 'module resolver must use canonical .vrn source mapping'
if grep -R -q 'join("mod.vrn")\|push("mod.vrn")' crates/compiler/src/source_loader.rs crates/compiler/src/source_loader; then
  fail 'R48 module resolution must not restore mod.vrn fallback'
fi
test -f crates/compiler/src/module_header.rs || fail 'missing compiler module header parser'
grep -Fq 'raw.strip_prefix("self::")' crates/compiler/src/module_header.rs || fail 'module parser must support self-relative prefixes'
grep -Fq 'raw.strip_prefix("super::")' crates/compiler/src/module_header.rs || fail 'module parser must support parent-relative prefixes'
grep -Fq 'raw.strip_prefix("crate::")' crates/compiler/src/module_header.rs || fail 'module parser must support crate-relative prefixes'
grep -Fq '`super::` cannot escape the package root' crates/compiler/src/module_header.rs || fail 'module parser must fail closed when super escapes package root'
grep -q 'pub(crate) fn qualify(namespace: &str, local: &str)' crates/compiler/src/module_namespace.rs || fail 'compiler must centralize symbol qualification'
grep -q 'pub(crate) fn resolve(namespace: &str, name: &str)' crates/compiler/src/module_namespace.rs || fail 'compiler must centralize namespace resolution'
grep -q 'fn cross_module_handler_reference_must_be_qualified' crates/compiler/src/tests/module_namespace_compile_tests.rs || fail 'R48 must test qualified cross-module handler references'
grep -q 'fn local_handler_reference_resolves_inside_its_module' crates/compiler/src/tests/module_namespace_compile_tests.rs || fail 'R48 must test local lexical module resolution'
grep -q 'fn module_symbols_stay_in_their_namespace' crates/compiler/src/tests/module_namespace_compile_tests.rs || fail 'R48 must test namespace isolation'
grep -q 'fn module_declarations_after_code_are_rejected' crates/compiler/src/tests/module_namespace_compile_tests.rs || fail 'R48 must reject late module declarations'
grep -q 'queries::articleBySlug' examples/starter-project/pages.vrn || fail 'starter example must use qualified query references'
grep -q 'pages::home' examples/starter-project/pages.vrn || fail 'starter example routes must use qualified page handlers'
grep -q 'Result<models::Article' examples/starter-project/queries.vrn || fail 'starter example query return models must be qualified'
max_lines crates/compiler/src/source_loader.rs 180
max_lines crates/compiler/src/source_loader/tree.rs 220
max_lines crates/compiler/src/source_loader/reexports.rs 140
max_lines crates/compiler/src/module_namespace.rs 80
max_lines crates/compiler/src/tests/module_namespace_compile_tests.rs 240
printf '%s\n' 'R48 module namespace and resolution verification passed'

# R49: positive examples are an explicit, complete contract and R48 namespaces have a dedicated example.
test -f tests/manifests/example-entrypoints.txt || fail 'missing positive example entrypoint manifest'
test -f examples/module-namespaces/main.vrn || fail 'missing module namespace example entrypoint'
test -f examples/module-namespaces/catalog.vrn || fail 'missing module namespace root module example'
test -f examples/module-namespaces/catalog/queries.vrn || fail 'missing nested module namespace query example'
test -f examples/module-namespaces/catalog/pages.vrn || fail 'missing nested module namespace page example'
grep -q '^mod catalog::queries;$' examples/module-namespaces/main.vrn || fail 'namespace example must declare nested query module explicitly'
grep -q 'Result<Vec<catalog::Product>, DbError>' examples/module-namespaces/catalog/queries.vrn || fail 'namespace example must qualify cross-module model type'
grep -q 'catalog::queries::recent(db)' examples/module-namespaces/catalog/pages.vrn || fail 'namespace example must qualify cross-module query call'
grep -q '=> catalog::pages::index' examples/module-namespaces/catalog/pages.vrn || fail 'namespace example route must qualify handler'
if grep -RIn --include='*.vrn' -E '^[[:space:]]*mod[[:space:]]+(\.\.?/|self::|super::|crate::)' examples/module-namespaces >/dev/null 2>&1; then
  fail 'namespace example must not use relative/self/super/crate module paths'
fi
positive_dirs=0
for dir in examples/*; do
  [ -d "$dir" ] || continue
  if find "$dir" -type f -name '*.vrn' -print -quit | grep -q .; then
    positive_dirs=$((positive_dirs + 1))
    grep -Eq "^${dir}/((app|main)\.vrn|app/main\.vrn)$" tests/manifests/example-entrypoints.txt || fail "positive example directory missing from manifest: $dir"
  fi
done
manifest_entries=$(grep -vE '^[[:space:]]*(#|$)' tests/manifests/example-entrypoints.txt | wc -l | tr -d ' ')
[ "$manifest_entries" -eq "$positive_dirs" ] || fail 'positive example manifest must contain exactly one entrypoint per positive example directory'
printf '%s\n' 'R49 example-suite coverage verification passed'

# R49.1: namespace-aware expression parser is the production entrypoint; legacy wrapper is test-only.
grep -q '^#\[cfg(test)\]$' crates/compiler/src/expression.rs || fail 'parse_expr compatibility wrapper must be test-only'
grep -q '^pub(super) fn parse_expr(input: &str, program: &Program)' crates/compiler/src/expression.rs || fail 'missing test-only parse_expr compatibility wrapper'
if grep -n '^use crate::expression::.*parse_expr.*parse_expr_in_namespace' crates/compiler/src/control_flow.rs >/dev/null 2>&1; then
  fail 'control_flow production imports must not include legacy parse_expr wrapper'
fi
grep -q '^#\[cfg(test)\]$' crates/compiler/src/control_flow.rs || fail 'control_flow test-only parse_expr import must be cfg(test) guarded'
printf '%s\n' 'R49.1 expression parser warning cleanup verification passed'

# R50: Markdown documentation must mirror the R48/R49 module namespace contract.
test -f docs/21-modules-and-namespaces.md || fail 'missing canonical module namespace Markdown'
test -f docs/hu/21-modules-slugs-project-layout.md || fail 'missing Hungarian module namespace Markdown'
grep -q 'Module declarations are top-level source-graph declarations' docs/21-modules-and-namespaces.md || fail 'English module docs must explain top-level source-graph declarations'
grep -q 'Directories are never scanned automatically' docs/21-modules-and-namespaces.md || fail 'English module docs must reject automatic directory discovery'
grep -q 'wildcard' docs/21-modules-and-namespaces.md || fail 'English module docs must document wildcard import rejection'
grep -q 'A `mod` top-level source-graph deklaráció' docs/hu/21-modules-slugs-project-layout.md || fail 'Hungarian module docs must explain top-level source-graph declarations'
grep -q 'nem járja be automatikusan' docs/hu/21-modules-slugs-project-layout.md || fail 'Hungarian module docs must reject automatic directory discovery'
grep -q 'pages::home' docs/hu/01-gyors-kezdes.md || fail 'Hungarian quick start must show qualified cross-module handler naming'
grep -q '^routes\.vrn$' docs/hu/22-domain-objects.md || fail 'domain-object example tree must include routes.vrn shown by mod routes'
grep -q 'examples/module-namespaces/' docs/README.md || fail 'documentation index must point to namespace compatibility example'
grep -q 'foo::bar.*foo/bar\.vrn' README.md || fail 'root README must state canonical nested module mapping'
if grep -q 'Future cleanup may replace the long tuple' docs/50-maintainability-and-clean-code.md; then
  fail 'maintainability docs must not describe completed StartupArgs migration as future work'
fi
printf '%s\n' 'R50 Markdown namespace synchronization verification passed'



# R51: LaTeX books mirror the current module namespace and compute builtin surface.
grep -q 'Module namespaces and cross-module calls' docs/book/chapters/04-language-and-http.tex || fail 'English book must document module namespace calls'
grep -q 'Modul-névterek és modulok közötti hívások' docs/book/hu/chapters/04-nyelv-http.tex || fail 'Hungarian book must document module namespace calls'
grep -q 'Expressions and arithmetic' docs/book/chapters/04-language-and-http.tex || fail 'English book must document arithmetic'
grep -q 'Kifejezések és aritmetika' docs/book/hu/chapters/04-nyelv-http.tex || fail 'Hungarian book must document arithmetic'
grep -q 'text.chars().count()' docs/book/chapters/04-language-and-http.tex || fail 'English book must document Rust-like string methods'
grep -q 'text.chars().count()' docs/book/hu/chapters/04-nyelv-http.tex || fail 'Hungarian book must document Rust-like string methods'
grep -q 'catalog::pages::index' docs/book/chapters/04-language-and-http.tex || fail 'English book must show qualified cross-module handler'
grep -q 'catalog::pages::index' docs/book/hu/chapters/04-nyelv-http.tex || fail 'Hungarian book must show qualified cross-module handler'
grep -q 'article::Article.show' docs/book/chapters/11-larger-applications.tex || fail 'English larger-app routes must qualify cross-module domain handler'
grep -q 'article::Article.show' docs/book/hu/chapters/11-nagyobb-alkalmazasok.tex || fail 'Hungarian larger-app routes must qualify cross-module domain handler'
printf '%s\n' 'R51 LaTeX language/reference synchronization verification passed'

# R52: numeric/string core completion keeps parser/type/runtime responsibilities explicit.
test -f crates/compiler/src/expression_parser.rs || fail 'R52 missing expression parser grammar owner'
test -f crates/compiler/src/math_builtin_types.rs || fail 'R52 missing compiler math builtin type owner'
test -f crates/compiler/src/string_builtin_types.rs || fail 'R52 missing compiler string builtin type owner'
grep -q 'Rem,' crates/language-core/src/ast.rs || fail 'R52 BinaryOp must include remainder'
grep -q 'ShiftLeft,' crates/language-core/src/ast.rs || fail 'R52 BinaryOp must include shifts'
grep -q 'LogicalAnd,' crates/language-core/src/ast.rs || fail 'R52 BinaryOp must include logical operators'
grep -q 'Ln,' crates/language-core/src/builtin.rs || fail 'R52 builtins must include logarithms'
grep -q 'Round,' crates/language-core/src/builtin.rs || fail 'R52 builtins must include rounding'
grep -q 'Substring,' crates/language-core/src/builtin.rs || fail 'R52 builtins must include substring'
grep -q 'Repeat,' crates/language-core/src/builtin.rs || fail 'R52 builtins must include repeat'
grep -q 'examples/numeric-operators/main.vrn' tests/manifests/example-entrypoints.txt || fail 'R52 numeric operator example must be compile-checked'
grep -q 'cleaned.to_lowercase()' examples/string-operations/main.vrn || fail 'R52 string example must exercise Rust-like string methods'
grep -q '8.0f32.log(2.0f32)' examples/numeric-operators/main.vrn || fail 'R52 numeric example must exercise Rust-like logarithm method'
grep -q 'short-circuit' docs/44-math-and-timing.md || fail 'R52 math docs must state short-circuit semantics'
grep -q 'substring(String, i64, i64)' docs/46-string-builtins.md || fail 'R52 string docs must document three-argument substring'
grep -Fq 'x.round()/x.floor()/x.ceil()' docs/book/chapters/04-language-and-http.tex || fail 'R52 English book must document rounding builtins'
grep -Fq 'x.round()/x.floor()/x.ceil()' docs/book/hu/chapters/04-nyelv-http.tex || fail 'R52 Hungarian book must document rounding builtins'
for file in crates/compiler/src/expression_parser.rs crates/compiler/src/math_builtin_types.rs crates/compiler/src/string_builtin_types.rs; do
  if grep -q '^use super::\*;' "$file"; then
    fail "R52 production module must use explicit imports: $file"
  fi
done
max_lines crates/compiler/src/expression.rs 280
max_lines crates/compiler/src/expression_parser.rs 330
max_lines crates/compiler/src/math_builtin_types.rs 140
max_lines crates/compiler/src/string_builtin_types.rs 150
printf '%s\n' 'R52 numeric/string core completion verification passed'

# R53: explicit semicolon statement terminators; newline is never a statement boundary.
grep -q 'simple statement must end with `;`' crates/compiler/src/source_syntax.rs || fail 'R53 missing required simple-statement terminator error'
grep -q 'return statement must end with `;`' crates/compiler/src/source_syntax.rs || fail 'R53 missing required return terminator error'
if grep -n "byte == b'\\n'" crates/compiler/src/source_syntax.rs | grep -q 'find_statement_end'; then
  fail 'R53 find_statement_end must not treat newline as a terminator'
fi
grep -q 'fn simple_statements_require_semicolons' crates/compiler/src/tests/statement_terminator_tests.rs || fail 'R53 missing missing-semicolon compiler test'
grep -q 'fn multiline_expression_uses_semicolon_not_newline_as_terminator' crates/compiler/src/tests/statement_terminator_tests.rs || fail 'R53 missing multiline-expression terminator test'
grep -q 'fn block_statements_do_not_require_trailing_semicolons' crates/compiler/src/tests/statement_terminator_tests.rs || fail 'R53 missing block terminator test'
test -f docs/56-statement-terminators.md || fail 'R53 missing English statement terminator documentation'
test -f docs/hu/55-statement-terminatorok.md || fail 'R53 missing Hungarian statement terminator documentation'
grep -q 'there is no automatic semicolon insertion' docs/56-statement-terminators.md || fail 'R53 English docs must reject automatic semicolon insertion'
grep -q 'nincs automatikus semicolon insertion' docs/hu/55-statement-terminatorok.md || fail 'R53 Hungarian docs must reject automatic semicolon insertion'
grep -q 'Statement terminators' docs/book/chapters/04-language-and-http.tex || fail 'R53 English book must document statement terminators'
grep -q 'Statement terminátorok' docs/book/hu/chapters/04-nyelv-http.tex || fail 'R53 Hungarian book must document statement terminators'
# Positive examples must not rely on newline termination for the common simple-statement forms.
if grep -RInE '^[[:space:]]*(let|set|authorize|flash|canonical slug)[[:space:]].*[^;[:space:]]$' examples --include='*.vrn' --exclude-dir=negative --exclude-dir=security >/dev/null 2>&1; then
  fail 'R53 positive examples contain simple statements without semicolons'
fi
if grep -RInE '\?[[:space:]]*$' examples --include='*.vrn' --exclude-dir=negative --exclude-dir=security >/dev/null 2>&1; then
  fail 'R53 positive examples contain transaction/query statements without semicolons'
fi
printf '%s\n' 'R53 explicit statement terminator verification passed'

# R54: explicit semicolon terminators for non-block top-level route declarations.
grep -q 'route declaration must end with `;`' crates/compiler/src/routes/route_scanner.rs || fail 'R54 route scanner must diagnose missing route semicolon'
grep -q 'tokens.get(arrow + 2).map(String::as_str) == Some(";")' crates/compiler/src/routes/route_scanner.rs || fail 'R54 route scanner must require semicolon after handler'
grep -q 'fn route_declarations_require_semicolons' crates/compiler/src/tests/statement_terminator_tests.rs || fail 'R54 missing route-semicolon rejection test'
grep -q 'fn multiline_route_is_terminated_by_semicolon_not_newline' crates/compiler/src/tests/statement_terminator_tests.rs || fail 'R54 missing multiline route terminator test'
grep -q 'route .*=> handler;' docs/56-statement-terminators.md || fail 'R54 English terminator docs must document semicolon-terminated routes'
grep -q 'route .*=> handler;' docs/hu/55-statement-terminatorok.md || fail 'R54 Hungarian terminator docs must document semicolon-terminated routes'
python3 - <<'PY'
from pathlib import Path
bad = []
for base in (Path('examples'), Path('docs/book')):
    for path in base.rglob('*'):
        if not path.is_file() or path.suffix not in {'.vrn', '.tex'}:
            continue
        in_route = False
        for line_no, line in enumerate(path.read_text().splitlines(), 1):
            stripped = line.lstrip()
            if stripped.startswith('route '):
                in_route = True
            if in_route and '=>' in stripped:
                code = stripped.split('//', 1)[0].rstrip()
                if not code.endswith(';'):
                    bad.append(f'{path}:{line_no}')
                in_route = False
if bad:
    raise SystemExit('R54 unterminated route declarations: ' + ', '.join(bad[:20]))
PY
printf '%s\n' 'R54 explicit route terminator verification passed'

# R54.2: declaration lexer owns explicit semicolon tokens; expression lexer stays statement-agnostic.
python3 - <<'PY'
from pathlib import Path
text = Path('crates/compiler/src/lexer.rs').read_text()
head, sep, tail = text.partition('pub(super) fn lex_expr')
if not sep:
    raise SystemExit('R54.2 lexer split point missing')
if "if c[i] == ';'" not in head or 'o.push(";".into());' not in head:
    raise SystemExit('R54.2 declaration lexer must emit an explicit semicolon token')
if "ExprToken::Semicolon" in tail:
    raise SystemExit('R54.2 expression lexer must not own statement-terminator tokens')
if "b';'" in tail and "ExprToken::Semi" not in tail:
    raise SystemExit('R54.2 expression semicolon may only be the Rust-like vec! repeat separator')
PY
grep -q 'fn semicolon_separates_let_and_return_statements' crates/compiler/src/tests/core_compile_tests.rs || fail 'R54.2 stale newline statement-boundary regression test name remains'
if grep -q 'fn newline_separates_let_and_return_statements' crates/compiler/src/tests/core_compile_tests.rs; then
  fail 'R54.2 newline must not be described as a statement separator'
fi
printf '%s\n' 'R54.2 declaration lexer semicolon verification passed'

# R54.3: each route scanner result is parsed exactly once; no token-index re-scan loop.
python3 - <<'PY'
from pathlib import Path
text = Path('crates/compiler/src/routes.rs').read_text()
start = text.index('pub(super) fn parse_routes')
end = text.index('\npub(super) fn parse_typed_binding', start)
body = text[start:end]
if 'while i < t.len()' in body:
    raise SystemExit('R54.3 parse_routes must not re-scan tokens inside a single scanner result')
if 'let i = 0;' not in body:
    raise SystemExit('R54.3 parse_routes must parse each scanner result from its first token')
if 't.get(i).map(String::as_str) != Some("route")' not in body:
    raise SystemExit('R54.3 parse_routes must assert the scanner/parser route boundary')
if 'i = c + 2' in body:
    raise SystemExit('R54.3 stale route token-index advancement remains')
PY
printf '%s\n' 'R54.3 route scanner/parser boundary verification passed'

# R54.4: interpreter-only runtime fixtures were removed with the native-only cutover.
printf '%s\n' 'R54.4 native-only runtime fixture cleanup verification passed'

# R55: Markdown language reference must track current numeric/string/terminator surface.
grep -Fq '## Language syntax essentials' README.md || fail 'R55 root README must expose current language syntax essentials'
grep -Fq 'docs/44-math-and-timing.md' README.md || fail 'R55 root README must link numeric/math reference'
grep -Fq 'docs/46-string-builtins.md' README.md || fail 'R55 root README must link string reference'
grep -Fq 'docs/56-statement-terminators.md' README.md || fail 'R55 root README must link terminator reference'
grep -Fq '## Core language reference' docs/README.md || fail 'R55 English docs index must expose a core language reference section'
grep -Fiq 'Numeric operators, f32 math and monotonic timing' docs/README.md || fail 'R55 English docs index must link numeric operators'
grep -Fq 'Rust-like string operations' docs/README.md || fail 'R55 English docs index must link Rust-like string operations'
grep -Fq 'Statement terminators' docs/README.md || fail 'R55 English docs index must link statement terminators'
grep -Fq '## V1 aktuális kifejezés- és string surface' docs/hu/README.md || fail 'R55 Hungarian docs index must expose current expression/string surface'
grep -Fq '`ln`, `log10`, `log`, `exp`, `pow`, `round`, `floor` és `ceil`' docs/hu/README.md || fail 'R55 Hungarian docs index must enumerate extended math builtins'
grep -Fq '`.replace()` és `.repeat()`' docs/hu/README.md || fail 'R55 Hungarian docs index must enumerate Rust-like string manipulation core'
grep -Fq 'A sortörés nem terminátor' docs/hu/README.md || fail 'R55 Hungarian docs index must document explicit semicolon boundary'
grep -Fq 'let bucket = id % 16;' docs/hu/03-nyelv-html.md || fail 'R55 Hungarian language basics must demonstrate modulo'
grep -Fq 'let visible = published && !deleted;' docs/hu/03-nyelv-html.md || fail 'R55 Hungarian language basics must demonstrate boolean logic'
grep -Fq 'cleaned.to_lowercase().replace' docs/hu/03-nyelv-html.md || fail 'R55 Hungarian language basics must demonstrate Rust-like string replace'
grep -Fq 'route catalogIndex GET "/catalog"' docs/56-statement-terminators.md || fail 'R55 English terminator docs must demonstrate multiline route terminator'
grep -Fq '=> catalog::pages::index;' docs/56-statement-terminators.md || fail 'R55 English route example must end in semicolon'
grep -Fq '=> catalog::pages::index;' docs/hu/55-statement-terminatorok.md || fail 'R55 Hungarian route example must end in semicolon'
grep -Fq 'substring(String, i64, i64) -> String' docs/46-string-builtins.md || fail 'R55 English string reference must include substring overload'
grep -Fq 'String::repeat(i64) -> String' docs/hu/45-string-muveletek.md || fail 'R55 Hungarian string reference must include Rust-like repeat'
grep -Fq 'f32.powf(f32) -> f32' docs/44-math-and-timing.md || fail 'R55 English math reference must include pow'
grep -Fq 'f32.ceil() -> f32' docs/hu/43-matematika-es-idomeres.md || fail 'R55 Hungarian math reference must include ceil'
printf '%s\n' 'R55 Markdown language reference verification passed'
# R55: representative legacy Markdown examples must also use explicit simple-statement terminators.
grep -Fq 'let healthy = true;' docs/hu/05-json-api-cors.md || fail 'R55 JSON API example must use explicit statement terminators'
grep -Fq 'let result = 40 + 2;' docs/hu/10-resource-profile.md || fail 'R55 resource-profile example must use explicit statement terminators'
grep -Fq 'return Ok(redirect("/products?page=1&pageSize=20"));' docs/hu/06-crud.md || fail 'R55 CRUD example must use explicit return terminator'
grep -Fq 'let savedPath = file.path;' docs/hu/09-upload-appfs.md || fail 'R55 upload example must use explicit statement terminators'
grep -Fq 'let gross = net + tax;' docs/hu/11-business-types.md || fail 'R55 business-type example must use explicit statement terminators'
printf '%s\n' 'R55 Markdown example terminator migration verification passed'

# R56: LaTeX language chapter must teach the current language surface as a web-development workflow.
grep -Fq '\chapter{Language essentials and HTTP boundaries}' docs/book/chapters/04-language-and-http.tex || fail 'R56 English book must use the web-developer-oriented language chapter'
grep -Fq '\section{Source structure and statement boundaries}' docs/book/chapters/04-language-and-http.tex || fail 'R56 English book must separate source/statement boundaries'
grep -Fq '\section{Typed computation for web application code}' docs/book/chapters/04-language-and-http.tex || fail 'R56 English book must separate typed computation'
grep -Fq '\subsection*{Practical arithmetic patterns}' docs/book/chapters/04-language-and-http.tex || fail 'R56 English book must include practical arithmetic guidance'
grep -Fq '\subsection*{Practical text-handling patterns}' docs/book/chapters/04-language-and-http.tex || fail 'R56 English book must include practical string guidance'
grep -Fq '\subsection*{Choosing the right tool in application code}' docs/book/chapters/04-language-and-http.tex || fail 'R56 English book must include a web-development decision table'
grep -Fq '\chapter{Nyelvi alapok és HTTP-határok}' docs/book/hu/chapters/04-nyelv-http.tex || fail 'R56 Hungarian book must use the web-developer-oriented language chapter'
grep -Fq '\section{Forrásstruktúra és statement-határok}' docs/book/hu/chapters/04-nyelv-http.tex || fail 'R56 Hungarian book must separate source/statement boundaries'
grep -Fq '\section{Typed számítás webalkalmazás-kódban}' docs/book/hu/chapters/04-nyelv-http.tex || fail 'R56 Hungarian book must separate typed computation'
grep -Fq '\subsection*{Gyakori webes aritmetikai minták}' docs/book/hu/chapters/04-nyelv-http.tex || fail 'R56 Hungarian book must include practical arithmetic guidance'
grep -Fq '\subsection*{Gyakori szövegkezelési minták}' docs/book/hu/chapters/04-nyelv-http.tex || fail 'R56 Hungarian book must include practical string guidance'
grep -Fq '\subsection*{Melyik eszközt válaszd alkalmazáskódban?}' docs/book/hu/chapters/04-nyelv-http.tex || fail 'R56 Hungarian book must include a web-development decision table'
grep -Fq 'let offset = (page - 1) * pageSize;' docs/book/chapters/04-language-and-http.tex || fail 'R56 English book must show a practical pagination arithmetic pattern'
grep -Fq 'let displayTag = tag.trim().replace("_", " ");' docs/book/chapters/04-language-and-http.tex || fail 'R56 English book must show deterministic replace usage'
grep -Fq 'let offset = (page - 1) * pageSize;' docs/book/hu/chapters/04-nyelv-http.tex || fail 'R56 Hungarian book must show a practical pagination arithmetic pattern'
grep -Fq 'let displayTag = tag.trim().replace("_", " ");' docs/book/hu/chapters/04-nyelv-http.tex || fail 'R56 Hungarian book must show deterministic replace usage'
printf '%s\n' 'R56 web-developer-oriented LaTeX language chapter verification passed'

# R57: LaTeX book sources share typography, retain current Velran syntax, and keep reference tables clean.
test -f docs/book/velran-book-style.tex || fail 'R57 shared LaTeX book style must exist'
grep -Fq '\input{velran-book-style}' docs/book/main.tex || fail 'R57 English book must use shared LaTeX style'
grep -Fq '\input{../velran-book-style}' docs/book/hu/main.tex || fail 'R57 Hungarian book must use shared LaTeX style'
grep -Fq '\newcolumntype{L}' docs/book/velran-book-style.tex || fail 'R57 shared style must define ragged-right longtable columns'
grep -Fq '\newcolumntype{Y}' docs/book/velran-book-style.tex || fail 'R57 shared style must define ragged-right tabularx columns'
if grep -R -E '^[[:space:]]*[0-9]+[[:space:]]+\\bottomrule' docs/book/chapters docs/book/hu/chapters --include='*.tex' >/dev/null; then
    fail 'R57 book reference tables must not contain stray numeric rows before bottomrule'
fi
grep -Fq 'let article = Article.byId(db, id)?;' docs/book/chapters/11-larger-applications.tex || fail 'R57 English larger-app example must use explicit statement terminators'
grep -Fq 'authorize article owner authorUsername or role Publisher;' docs/book/chapters/11-larger-applications.tex || fail 'R57 English authorization example must use explicit statement terminator'
grep -Fq 'return Ok(redirect(adminArticles()));' docs/book/chapters/11-larger-applications.tex || fail 'R57 English return example must use explicit statement terminator'
grep -Fq 'let article = Article.byId(db, id)?;' docs/book/hu/chapters/11-nagyobb-alkalmazasok.tex || fail 'R57 Hungarian larger-app example must use explicit statement terminators'
grep -Fq 'velran-webfejlesztoknek-hu.pdf' docs/book/hu/README.md || fail 'R57 Hungarian book README must name the actual PDF output'
printf '%s\n' 'R57 LaTeX book audit verification passed'
grep -Fq '\path{examples/json-api/app.vrn}' docs/book/chapters/09-json-api-and-cors.tex || fail 'R57 English JSON example path must stay inline to avoid an orphan end page'
grep -Fq '\path{examples/json-api/app.vrn}' docs/book/hu/chapters/09-json-api-cors.tex || fail 'R57 Hungarian JSON example path must stay inline to keep editions structurally aligned'

"$ROOT/tools/check-pure-native-kernels.sh"
"$(dirname "$0")/check-direct-typed-kernel.sh"

"$(dirname "$0")/check-counted-loop-fuel-hoisting.sh"

"$ROOT/tools/check-crate-consolidation.sh"

"$(dirname -- "$0")/check-native-runtime-module-scope.sh"

"$(dirname -- "$0")/check-provider-features.sh"
"$(dirname -- "$0")/check-f32-expression-hoisting.sh"
"$(dirname -- "$0")/check-target-cpu.sh"

./tools/check-sin-cos-fusion.sh

"$(dirname "$0")/check-rust-syntax-convergence-iteration5.sh"

"$(dirname "$0")/check-rust-stdlib-surface.sh"

"$(dirname "$0")/check-rust-script-framework-convergence.sh"

"$(dirname "$0")/check-rust-first-surface.sh"

"$(dirname -- "$0")/check-string-map-fastpaths.sh"

"$(dirname -- "$0")/check-direct-typed-scalar-kernel.sh"

"$(dirname "$0")/check-bounded-json-boundary.sh"

"$(dirname "$0")/check-json-typed-request.sh"

"$(dirname "$0")/check-verified-pure-functions.sh"
"$(dirname "$0")/check-pure-fn-typed-return.sh"
"$(dirname "$0")/check-pure-fn-borrowed-string.sh"
"$(dirname "$0")/check-pure-function-shard-deps.sh"

tools/check-pure-fn-string-list.sh

"$(dirname "$0")/check-pure-composite-type-foundation.sh"

tools/check-pure-typed-struct-layout.sh

"$(dirname "$0")/check-pure-tagged-sum.sh"

"$ROOT/tools/check-pure-sum-methods.sh"

"$ROOT/tools/check-generic-postfix-parser.sh"

"$ROOT/tools/check-typed-data-fastpaths.sh"
"$ROOT/tools/check-typed-query-form.sh"

"$ROOT/tools/check-typed-multipart.sh"

"$ROOT/tools/check-formal-effects-resources.sh"

bash tools/check-web-security-hardening.sh
bash tools/check-release-stabilization.sh

"$ROOT/tools/check-deployment-build-identity.sh"

"$ROOT/tools/check-sum-expr-exhaustiveness-hotfix.sh"

"$ROOT/tools/check-compiler-lifetime-native-pure-import-hotfix.sh"

"$ROOT/tools/check-compiler-call-boundary-hotfix.sh"

"$ROOT/tools/check-pure-struct-field-scope-hotfix.sh"

"$ROOT/tools/check-native-binary-cache-fastpath.sh"

"$ROOT/tools/check-release-compile-clean.sh"

"$ROOT/tools/check-native-cache-reuse-integration.sh"

"$ROOT/tools/check-performance-regression-tooling.sh"

"$ROOT/tools/check-adversarial-fuzz-regressions.sh"

"$ROOT/tools/check-reproducible-release-recovery.sh"

"$ROOT/tools/check-native-build-test-scope-hotfix.sh"

./tools/check-commonmark.sh
./tools/check-editor-syntax.sh
