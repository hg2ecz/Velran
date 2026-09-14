use crate::server_config_file::DomainRuntime;
use crate::server_errors::StartupError;
use crate::{AuthRuntime, TlsCliConfig, WebSecurityCliConfig};
use language_core::{RouteAuth, ServerConfig};
use std::path::Path;
use std::sync::Arc;

pub(super) fn validate_replay_guard_requirements(
    domains: &[Arc<DomainRuntime>],
    redis_available: bool,
    web: &WebSecurityCliConfig,
) -> Result<(), StartupError> {
    let uses_idempotency = domains
        .iter()
        .any(|domain| domain.program.routes.iter().any(|route| route.idempotent));
    let uses_webhooks = domains.iter().any(|domain| {
        domain
            .program
            .routes
            .iter()
            .any(|route| matches!(route.auth, RouteAuth::Webhook(_)))
    });
    if (uses_idempotency || uses_webhooks) && !redis_available {
        return Err(StartupError::invalid(
            "application uses replay-protected idempotency/webhook routes but Redis is not configured; replay protection is fail-closed and has no memory fallback",
        ));
    }
    if uses_webhooks {
        let Some(root) = web.webhook_secrets_dir.as_deref() else {
            return Err(StartupError::invalid(
                "application declares verified webhook routes but web.webhook_secrets_dir is not configured",
            ));
        };
        let meta = std::fs::symlink_metadata(root)
            .map_err(|_| StartupError::invalid("web.webhook_secrets_dir is unavailable"))?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err(StartupError::invalid(
                "web.webhook_secrets_dir must be a real directory, not a symlink",
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_auth_requirements(
    domains: &[Arc<DomainRuntime>],
    auth_runtime: &AuthRuntime,
) -> Result<(), StartupError> {
    let protected_routes = domains.iter().any(|domain| {
        domain
            .program
            .routes
            .iter()
            .any(|route| !matches!(route.auth, RouteAuth::Public | RouteAuth::Webhook(_)))
    });
    if protected_routes && auth_runtime.ldap.is_none() && auth_runtime.local.is_none() {
        return Err(StartupError::invalid(
            "application declares protected routes, but no authentication backend is configured",
        ));
    }
    Ok(())
}

pub(super) fn validate_transport_configuration(
    multi_domain: bool,
    tls: &TlsCliConfig,
    config: &ServerConfig,
    web: &WebSecurityCliConfig,
    unix_socket: Option<&Path>,
    behind_proxy: bool,
    tls_enabled: bool,
) -> Result<bool, StartupError> {
    if multi_domain && tls.public_host.is_some() {
        return Err(StartupError::invalid(
            "multi-domain mode derives allowed public hosts from [[domains]]; do not configure tls.public_host/--public-host",
        ));
    }
    if behind_proxy && tls_enabled {
        return Err(StartupError::invalid(
            "server.behind_proxy/--behind-proxy cannot be combined with backend TLS; terminate TLS at the trusted reverse proxy",
        ));
    }
    #[cfg(not(unix))]
    if unix_socket.is_some() {
        return Err(StartupError::invalid(
            "server.unix_socket is only supported on Unix platforms",
        ));
    }
    if unix_socket.is_some() && !behind_proxy {
        return Err(StartupError::invalid(
            "server.unix_socket requires server.behind_proxy=true so forwarding headers have explicit proxy semantics",
        ));
    }
    if behind_proxy
        && unix_socket.is_none()
        && (!config.listen.ip().is_loopback() || web.trusted_proxy_cidrs.is_empty())
    {
        return Err(StartupError::invalid(
            "TCP behind-proxy mode requires a loopback listener and at least one explicit web.trusted_proxy_cidrs entry",
        ));
    }
    let reverse_proxy_https = !tls_enabled
        && !config.insecure_dev_cookies
        && behind_proxy
        && (unix_socket.is_some()
            || (config.listen.ip().is_loopback() && !web.trusted_proxy_cidrs.is_empty()))
        && (tls.public_host.is_some() || multi_domain);
    if !tls_enabled && !config.insecure_dev_cookies && !reverse_proxy_https {
        return Err(StartupError::invalid(
            "plain HTTP is development-only unless velran-server runs in explicit behind-proxy mode with a Unix socket or loopback trusted proxy; otherwise configure TLS or pass --insecure-dev-cookies explicitly",
        ));
    }
    if tls.http_redirect_listen.is_some() && !tls_enabled {
        return Err(StartupError::invalid(
            "--http-redirect-listen requires HTTPS/TLS configuration",
        ));
    }
    if tls.http_redirect_listen.is_some() && multi_domain {
        return Err(StartupError::invalid(
            "multi-domain mode does not use the single-host HTTP redirect listener; redirect at the reverse proxy or run separate redirects",
        ));
    }
    if tls.http_redirect_listen.is_some() && tls.public_host.is_none() {
        return Err(StartupError::invalid(
            "--http-redirect-listen requires --public-host",
        ));
    }
    if tls_enabled && !config.insecure_dev_cookies && tls.public_host.is_none() && !multi_domain {
        return Err(StartupError::invalid(
            "production HTTPS requires --public-host for Host/Origin pinning",
        ));
    }
    Ok(reverse_proxy_https)
}
