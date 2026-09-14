use crate::auth_http::{parse_cookie, session_cookie_name};
use crate::http_io::HttpRequest;
use crate::operations::serve_health_endpoint;
use crate::server_config_file::{DomainRuntime, HostingRuntime};
use crate::static_delivery::{serve_media_image, serve_static_asset};
use crate::tls_support::request_public_host;
use crate::web_security::apply_cors_headers;
use crate::{AuthRuntime, LifecycleCliConfig, Response, StaticAssets, WebSecurityCliConfig};
use auth::{AuthError, SessionBackend, SessionSnapshot};
use data::{Database, RedisStore};
use language_core::Program;
use observability::Metrics;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use storage::AppFs;
use tokio::sync::OwnedSemaphorePermit;
use tokio::time::timeout;

pub(super) struct EarlyResponse {
    pub(super) response: Response,
    pub(super) route_label: &'static str,
    pub(super) keep_alive: bool,
}

pub(super) async fn dispatch_early_request(
    path: &str,
    method: &str,
    keep_alive: bool,
    lifecycle: &LifecycleCliConfig,
    database: Option<&Database>,
    redis: Option<&RedisStore>,
    program: &Program,
    domain_namespace: &str,
    appfs: Option<&AppFs>,
    request: &HttpRequest,
    max_image_pixels: u64,
    static_assets: Option<&StaticAssets>,
    request_origin: Option<&str>,
    web: &WebSecurityCliConfig,
    metrics: &Metrics,
) -> Option<EarlyResponse> {
    let response_keep_alive = keep_alive && matches!(method, "GET" | "HEAD");
    if path == lifecycle.live_path.as_str() || path == lifecycle.ready_path.as_str() {
        let response = serve_health_endpoint(path, method, lifecycle, database, redis).await;
        if path == lifecycle.ready_path.as_str() && response.status >= 500 {
            metrics.inc_readiness_failures();
        }
        return Some(EarlyResponse {
            response,
            route_label: "__health__",
            keep_alive: response_keep_alive,
        });
    }
    if let Some(response) = crate::dev_compile_error::response(domain_namespace, method) {
        return Some(EarlyResponse {
            response,
            route_label: "__dev_compile_error__",
            keep_alive: false,
        });
    }
    if path.starts_with("/__velran/media/") {
        return Some(EarlyResponse {
            response: serve_media_image(program, appfs, request, path, max_image_pixels).await,
            route_label: "__media__",
            keep_alive: response_keep_alive,
        });
    }
    if let Some(static_assets) = static_assets {
        if path.starts_with(&static_assets.url_prefix) {
            let mut response = serve_static_asset(static_assets, request, path).await;
            apply_cors_headers(&mut response, request_origin, web);
            return Some(EarlyResponse {
                response,
                route_label: "__static__",
                keep_alive: response_keep_alive,
            });
        }
    }
    None
}

pub(super) async fn resolve_session(
    request: &HttpRequest,
    config: &language_core::ServerConfig,
    sessions: &SessionBackend,
    auth_runtime: &AuthRuntime,
) -> Result<(SessionSnapshot, bool), AuthError> {
    let cookie_name = session_cookie_name(config);
    let existing = request
        .header("cookie")
        .and_then(|value| parse_cookie(value, cookie_name));
    let (mut session, mut is_new) = match existing {
        Some(id) => match sessions.get(id).await? {
            Some(session) => (session, false),
            None => (sessions.create().await?, true),
        },
        None => (sessions.create().await?, true),
    };

    if let Some(principal) = session.principal.as_deref() {
        let generation_is_current = if let Some(local) = auth_runtime.local.as_ref() {
            matches!(
                local.session_generation(principal).await,
                Ok(Some(generation)) if generation == session.auth_generation
            )
        } else {
            auth_runtime.mapped_claims_generation(principal) == session.auth_generation
        };
        if !generation_is_current {
            let _ = sessions.invalidate(&session.id).await;
            session = sessions.create().await?;
            is_new = true;
        }
    }
    Ok((session, is_new))
}

pub(super) struct SelectedDomain {
    pub(super) domain: Arc<DomainRuntime>,
    pub(super) multi_domain: bool,
    pub(super) matched_host: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AdmissionError {
    QueueFull,
    QueueTimeout,
    Closed,
}

pub(super) fn select_domain(
    hosting: &Arc<RwLock<HostingRuntime>>,
    host_header: Option<&str>,
) -> Result<Option<SelectedDomain>, &'static str> {
    let matched_host = request_public_host(host_header);
    let hosting = hosting
        .read()
        .map_err(|_| "hosting runtime lock poisoned")?;
    let multi_domain = !hosting.domains.is_empty();
    let domain = if multi_domain {
        matched_host
            .as_ref()
            .and_then(|host| hosting.domains.get(host).cloned())
    } else {
        hosting.default.as_ref().cloned()
    };
    Ok(domain.map(|domain| SelectedDomain {
        domain,
        multi_domain,
        matched_host,
    }))
}

pub(super) async fn admit_domain_request(
    domain: &Arc<DomainRuntime>,
) -> Result<OwnedSemaphorePermit, AdmissionError> {
    if let Ok(permit) = Arc::clone(&domain.request_slots).try_acquire_owned() {
        return Ok(permit);
    }

    let queue_permit = Arc::clone(&domain.queue_slots)
        .try_acquire_owned()
        .map_err(|_| AdmissionError::QueueFull)?;
    let acquired = timeout(
        Duration::from_millis(domain.queue_timeout_ms),
        Arc::clone(&domain.request_slots).acquire_owned(),
    )
    .await;
    drop(queue_permit);

    match acquired {
        Ok(Ok(permit)) => Ok(permit),
        Ok(Err(_)) => Err(AdmissionError::Closed),
        Err(_) => Err(AdmissionError::QueueTimeout),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_errors_are_distinct() {
        assert_ne!(AdmissionError::QueueFull, AdmissionError::QueueTimeout);
        assert_ne!(AdmissionError::QueueTimeout, AdmissionError::Closed);
    }
}
