use auth::{AuthAbuseLimiter, LdapConfig, LocalUserStore, TotpReplayGuard};
use data::RedisStore;
use ipnet::IpNet;
use language_core::Route;
use observability::{
    AuditEvent, Metrics, RequestLog, RequestTimer, access_log, audit_log, init_logging, json_line,
    server_event, utc_timestamp,
};
use resource_limits::apply as apply_resource_limits;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use storage::AppFs;

#[derive(Clone)]
struct TlsCliConfig {
    cert_file: Option<PathBuf>,
    key_file: Option<PathBuf>,
    handshake_timeout_ms: u64,
    http_redirect_listen: Option<std::net::SocketAddr>,
    public_host: Option<String>,
}

impl Default for TlsCliConfig {
    fn default() -> Self {
        Self {
            cert_file: None,
            key_file: None,
            handshake_timeout_ms: 5000,
            http_redirect_listen: None,
            public_host: None,
        }
    }
}

#[derive(Clone, Default)]
struct WebSecurityCliConfig {
    trusted_proxy_cidrs: Vec<IpNet>,
    allow_missing_origin: bool,
    cors_origins: Vec<String>,
    cors_allow_credentials: bool,
    webhook_secrets_dir: Option<PathBuf>,
    egress_policy_file: Option<PathBuf>,
}

#[derive(Clone)]
struct StorageCliConfig {
    data_root: Option<PathBuf>,
    fs_mode: String,
    max_upload_bytes: u64,
    max_image_pixels: u64,
}
impl Default for StorageCliConfig {
    fn default() -> Self {
        Self {
            data_root: None,
            fs_mode: "rwc".into(),
            max_upload_bytes: 16 * 1024 * 1024,
            max_image_pixels: 40_000_000,
        }
    }
}

#[derive(Clone)]
struct StaticAssetsCliConfig {
    root: Option<PathBuf>,
    url_prefix: String,
    max_asset_bytes: u64,
    regular_max_age_secs: u64,
    immutable_max_age_secs: u64,
    precompressed: bool,
}
impl Default for StaticAssetsCliConfig {
    fn default() -> Self {
        Self {
            root: None,
            url_prefix: "/assets/".into(),
            max_asset_bytes: 8 * 1024 * 1024,
            regular_max_age_secs: 300,
            immutable_max_age_secs: 31_536_000,
            precompressed: true,
        }
    }
}

#[derive(Clone)]
struct StaticAssets {
    fs: AppFs,
    url_prefix: String,
    regular_max_age_secs: u64,
    immutable_max_age_secs: u64,
    precompressed: bool,
}

#[derive(Clone, Default)]
struct AuthCliConfig {
    redis_url: Option<String>,
    allow_insecure_redis: bool,
    ldap_url: Option<String>,
    ldap_search_base: Option<String>,
    ldap_username_attribute: String,
    ldap_bind_dn: Option<String>,
    ldap_bind_password: Option<String>,
    totp_secrets_file: Option<PathBuf>,
    roles_file: Option<PathBuf>,
    memberships_file: Option<PathBuf>,
    local_auth_db_url: Option<String>,
    require_totp: bool,
    login_max_attempts: u32,
    login_principal_max_attempts: u32,
    login_source_max_attempts: u32,
    mfa_max_attempts: u32,
    login_window_secs: u64,
}

struct AuthRuntime {
    ldap: Option<LdapConfig>,
    local: Option<LocalUserStore>,
    totp_secrets: HashMap<String, Vec<u8>>,
    roles: HashMap<String, Vec<String>>,
    memberships: HashMap<String, Vec<auth::TenantId>>,
    require_totp: bool,
    redis: Option<RedisStore>,
    local_totp: TotpReplayGuard,
    limiter: AuthAbuseLimiter,
}

#[derive(Clone)]
struct LifecycleCliConfig {
    live_path: String,
    ready_path: String,
    dependency_timeout_ms: u64,
    shutdown_grace_ms: u64,
}
impl Default for LifecycleCliConfig {
    fn default() -> Self {
        Self {
            live_path: "/health/live".into(),
            ready_path: "/health/ready".into(),
            dependency_timeout_ms: 1000,
            shutdown_grace_ms: 30_000,
        }
    }
}

#[derive(Clone, Default)]
struct ObservabilityCliConfig {
    metrics_listen: Option<std::net::SocketAddr>,
    allow_public_metrics: bool,
    access_log: bool,
}

#[derive(Clone)]
struct CacheCliConfig {
    max_ttl_secs: u64,
    max_entries: usize,
    max_bytes: usize,
    allow_memory: bool,
    singleflight_wait_timeout_ms: u64,
}
impl Default for CacheCliConfig {
    fn default() -> Self {
        Self {
            max_ttl_secs: 3600,
            max_entries: 10_000,
            max_bytes: 64 * 1024 * 1024,
            allow_memory: false,
            singleflight_wait_timeout_ms: 5_000,
        }
    }
}

mod app_execution;
mod auth_claims;
mod auth_http;
mod auth_observability;
mod auth_setup;
mod backend_support;
mod bootstrap_config;
mod build_identity;
mod connection;
mod connection_dispatch;
mod connection_finalize;
mod dev_compile_error;
mod http_io;
mod idempotency;
mod membership_setup;
mod native_config;
mod native_host;
mod native_runtime;
mod operations;
mod presentation;
mod rate_limit;
mod request_input;
mod request_json;
mod request_pipeline;
mod resource_limits;
mod resource_profile_bootstrap;
mod response_headers;
mod response_kind;
mod runtime_error_logging;
mod security_alerts;
mod security_headers;
mod server_config_file;
mod server_errors;
mod session_cookie;
mod source_reload;
mod source_reload_candidate;
mod static_delivery;
mod tls_support;
mod typed_json_response;
mod web_security;
mod webhook_verification;
use http_io::Response;
use operations::install_panic_logging_hook;
use presentation::endpoint_error;
use rate_limit::RouteRateLimiter;
use server_errors::{ClockError, ReservedPathError};

fn unix_secs() -> Result<u64, ClockError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(ClockError::BeforeUnixEpoch)
}
fn public_cache_key(
    domain_namespace: &str,
    route: &Route,
    generation: u64,
    path: &str,
    query: &[(String, String)],
    json: bool,
) -> String {
    let mut q = query.to_vec();
    q.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut raw = format!(
        "v1\n{}\n{}\n{}\n{}\n{}\n",
        domain_namespace,
        route.name,
        generation,
        path,
        if json { "json" } else { "html" }
    );
    for (k, v) in q {
        raw.push_str(&k.len().to_string());
        raw.push(':');
        raw.push_str(&k);
        raw.push('=');
        raw.push_str(&v.len().to_string());
        raw.push(':');
        raw.push_str(&v);
        raw.push('\n');
    }
    stable_key_hash(&raw)
}

fn stable_key_hash(v: &str) -> String {
    let digest = Sha256::digest(v.as_bytes());
    let mut out = String::with_capacity(32);
    for b in &digest[..16] {
        use std::fmt::Write;
        let _ = write!(out, "{b:02x}");
    }
    out
}

fn main() {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    if raw_args == ["--print-build-id"] {
        println!("{}", build_identity::current());
        return;
    }
    let parsed = match cli::parse_args() {
        Ok(v) => v,
        Err(err) => {
            eprintln!("velran-server error: {err}");
            std::process::exit(2);
        }
    };
    if let Err(err) = init_logging(parsed.log_config.clone()) {
        eprintln!("velran-server error: logging setup failed: {err}");
        std::process::exit(2);
    }
    install_panic_logging_hook();
    if let Err(err) = apply_resource_limits(&parsed.resource_limits) {
        server_event(
            "error",
            "resource_limit_setup_failed",
            "startup",
            &err.to_string(),
        );
        std::process::exit(2);
    }
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(v) => v,
        Err(err) => {
            server_event(
                "error",
                "runtime_create_failed",
                "startup",
                &err.to_string(),
            );
            std::process::exit(1);
        }
    };
    let debug_compile_errors = parsed.source_reload.debug_compile_errors;
    let debug_app = parsed.app.clone();
    if let Err(err) = runtime.block_on(startup::run(parsed)) {
        server_event("error", "server_failed", "server", &err.to_string());
        if let Some(rustc) = err.rustc_diagnostics() {
            server_event("error", "startup_rustc_diagnostics", "compiler", rustc);
        }
        if debug_compile_errors {
            if let Some(compile_error) = err.compiler_error() {
                let diagnostic = compile_error.render_debug(&debug_app);
                eprintln!("\nVelran startup compilation failed.");
                eprint!("{diagnostic}");
                eprintln!();
                server_event(
                    "error",
                    "startup_compile_diagnostics",
                    "compiler",
                    &diagnostic,
                );
            } else if let Some(rustc) = err.rustc_diagnostics() {
                eprintln!("\nVelran startup native compilation failed.");
                eprintln!("{rustc}");
            } else {
                eprintln!("velran-server error: {err}");
            }
        }
        std::process::exit(1);
    }
}

mod cli;
mod cli_config_apply;
mod cli_effective;
mod cli_finalize;
mod cli_help;
mod cli_json_limits;
mod cli_overrides;
mod cli_scan;
mod http_dispatch;
mod http_system_values;
mod outbound_runtime;
mod outbound_startup;
mod production_security;
mod reload_security;
mod request_target_security;
mod startup;
mod startup_args;
mod startup_lifecycle;
mod startup_security;
mod startup_services;
mod startup_transport;

fn validate_reserved_path(raw: &str) -> Result<String, ReservedPathError> {
    if !raw.starts_with('/')
        || raw.len() > 128
        || raw.contains('?')
        || raw.contains('#')
        || raw.contains("//")
        || raw.contains("..")
        || raw.bytes().any(|b| b <= 0x20 || b == 0x7f)
    {
        return Err(ReservedPathError::new(raw));
    }
    Ok(raw.to_string())
}

fn route_matches_exact_path(route: &language_core::Route, path: &str) -> bool {
    let wanted: Vec<&str> = path
        .trim_start_matches('/')
        .split('/')
        .filter(|v| !v.is_empty())
        .collect();
    if route.segments.len() != wanted.len() {
        return false;
    }
    route
        .segments
        .iter()
        .zip(wanted)
        .all(|(segment, value)| match segment {
            language_core::RouteSegment::Static(v) => v == value,
            language_core::RouteSegment::Param { .. } => !value.is_empty(),
        })
}

async fn check_route_rate_limit(
    limiter: &RouteRateLimiter,
    route: &Route,
    effective_peer: &str,
    principal: Option<&str>,
    json_api: bool,
) -> Option<Response> {
    let policy = route.rate_policy.as_deref()?;
    match limiter
        .check(policy, &route.name, effective_peer, principal)
        .await
    {
        Ok((true, _)) => None,
        Ok((false, retry_after)) => {
            let mut response = endpoint_error(
                json_api,
                429,
                "Too Many Requests",
                "rate_limited",
                b"rate limit exceeded\n",
            );
            response.push_header(
                crate::response_headers::HeaderName::RetryAfter,
                retry_after.to_string(),
            );
            Some(response)
        }
        Err(_) => Some(endpoint_error(
            json_api,
            503,
            "Service Unavailable",
            "rate_limiter_unavailable",
            b"rate limiter unavailable\n",
        )),
    }
}

fn security_audit_classification(status: u16, body: &[u8]) -> Option<(&'static str, &'static str)> {
    let contains = |needle: &[u8]| {
        body.windows(needle.len())
            .any(|w| w.eq_ignore_ascii_case(needle))
    };
    match status {
        400 if contains(b"invalid forwarding headers") => Some(("proxy", "invalid_forwarding")),
        401 if contains(b"webhook") => Some(("webhook", "verification_denied")),
        401 => Some(("auth", "unauthorized")),
        403 if contains(b"csrf") => Some(("csrf", "denied")),
        403 if contains(b"cors") => Some(("cors", "denied")),
        403 if contains(b"origin") || contains(b"cross-site") => Some(("origin", "denied")),
        403 => Some(("policy", "forbidden")),
        421 => Some(("host", "mismatch")),
        426 => Some(("transport", "https_required")),
        409 if contains(b"idempotency") || contains(b"idempotent") => {
            Some(("idempotency", "conflict"))
        }
        429 => Some(("rate_limit", "denied")),
        503 if contains(b"resource_limit") => Some(("resource", "exhausted")),
        _ => None,
    }
}

fn observe_response(
    response: &mut Response,
    request_id: &str,
    method: &str,
    route: &str,
    client_ip: &str,
    bytes_in: u64,
    timer: &RequestTimer,
    metrics: &Metrics,
    access_enabled: bool,
) {
    if !response.has_header(crate::response_headers::HeaderName::XRequestId) {
        response.push_header(crate::response_headers::HeaderName::XRequestId, request_id);
    }
    let duration = timer.elapsed();
    let bytes_out = if response.suppress_body {
        0
    } else {
        response.body.len() as u64
    };
    metrics.record_response(route, response.status, duration, bytes_in, bytes_out);
    if matches!(response.status, 401 | 403) {
        metrics.inc_auth_failures();
    }
    if response.status == 403
        && response
            .body
            .windows(4)
            .any(|w| w.eq_ignore_ascii_case(b"csrf"))
    {
        metrics.inc_csrf_failures();
    }
    if matches!(response.status, 403 | 421 | 426)
        || (response.status == 400
            && response
                .body
                .windows(b"invalid forwarding headers".len())
                .any(|w| w.eq_ignore_ascii_case(b"invalid forwarding headers")))
    {
        metrics.inc_policy_denials();
    }
    if response.status == 429 {
        metrics.inc_rate_limit_denials();
    }
    if response.status == 408
        || response
            .body
            .windows(7)
            .any(|w| w.eq_ignore_ascii_case(b"timeout"))
    {
        metrics.inc_request_timeouts();
    }
    if response
        .body
        .windows(14)
        .any(|w| w.eq_ignore_ascii_case(b"resource_limit"))
        || response
            .body
            .windows(15)
            .any(|w| w.eq_ignore_ascii_case(b"execution limit"))
    {
        metrics.inc_budget_exceeded();
    }
    if access_enabled {
        if let Ok(line) = json_line(&RequestLog {
            schema_version: 1,
            timestamp: utc_timestamp(),
            event: "http_request",
            request_id,
            method,
            route,
            status: response.status,
            duration_ms: duration.as_millis(),
            client_ip,
            bytes_in,
            bytes_out,
        }) {
            access_log(&line);
        }
    }
    let security = security_audit_classification(response.status, &response.body);
    if let Some((category, outcome)) = security {
        if let Ok(line) = json_line(&AuditEvent {
            schema_version: 1,
            timestamp: utc_timestamp(),
            event: "security_audit",
            request_id,
            category,
            action: "request",
            outcome,
            detail: route,
        }) {
            audit_log(&line);
        }
        crate::security_alerts::observe(category, "request", outcome, client_ip, request_id, route);
    }
}

#[cfg(test)]
mod main_tests;
