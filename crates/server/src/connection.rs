use crate::bootstrap_config::PublicPageCache;
use crate::connection_dispatch;
use crate::connection_finalize;
use crate::http_io::{HttpRequest, Response, read_request_head, write_response_with_timeout};
use crate::presentation::read_error_response;
use crate::rate_limit::RouteRateLimiter;
use crate::request_pipeline::{
    AdmissionError, admit_domain_request, dispatch_early_request, resolve_session, select_domain,
};
use crate::server_config_file::HostingRuntime;
use crate::server_errors::ConnectionError;
use crate::tls_support::host_matches_public;
use crate::web_security::{effective_client_ip, effective_request_https};
use crate::{
    AuthRuntime, LifecycleCliConfig, ObservabilityCliConfig, WebSecurityCliConfig, observe_response,
};
use auth::SessionBackend;
use data::{Database, RedisStore};
use language_core::{HttpMethod, ServerConfig};
use observability::{Metrics, RequestTimer, new_request_id};
use runtime::route_meta_for_request;
use std::net::IpAddr;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::time::timeout;

pub(super) struct ConnectionServices<'a> {
    pub(super) hosting: &'a Arc<RwLock<HostingRuntime>>,
    pub(super) config: &'a ServerConfig,
    pub(super) sessions: &'a SessionBackend,
    pub(super) database: Option<&'a Database>,
    pub(super) auth_runtime: &'a AuthRuntime,
    pub(super) lifecycle: &'a LifecycleCliConfig,
    pub(super) route_rate_limiter: &'a RouteRateLimiter,
    pub(super) public_cache: &'a PublicPageCache,
    pub(super) idempotency_redis: Option<&'a RedisStore>,
    pub(super) outbound: Option<&'a dyn runtime::OutboundRuntime>,
    pub(super) metrics: &'a Metrics,
    pub(super) observability: &'a ObservabilityCliConfig,
    pub(super) web: &'a WebSecurityCliConfig,
}

pub(super) async fn handle_connection<S>(
    mut stream: S,
    services: &ConnectionServices<'_>,
    peer_ip: IpAddr,
    is_tls: bool,
    legacy_expected_host: Option<&str>,
) -> Result<(), ConnectionError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    let _connection_guard = services.metrics.connection_guard();
    let mut buffer = Vec::with_capacity(8192);
    for request_index in 0..services.config.max_requests_per_connection {
        let head = match timeout(
            Duration::from_millis(services.config.read_timeout_ms),
            read_request_head(&mut stream, &mut buffer, services.config),
        )
        .await
        {
            Err(_) => {
                write_response_with_timeout(
                    &mut stream,
                    services.config,
                    Response::text(408, "Request Timeout", b"request timeout\n"),
                    false,
                    is_tls,
                )
                .await?;
                break;
            }
            Ok(Ok(Some(v))) => v,
            Ok(Ok(None)) => break,
            Ok(Err(err)) => {
                let response = read_error_response(err);
                write_response_with_timeout(&mut stream, services.config, response, false, is_tls)
                    .await?;
                break;
            }
        };
        let request_timer = RequestTimer::start();
        let request_id = new_request_id();
        let observed_method = head.method.clone();
        let observed_bytes_in = head.content_length as u64;
        let request_stub = HttpRequest {
            method: head.method.clone(),
            target: head.target.clone(),
            headers: head.headers.clone(),
            body: Vec::new(),
        };
        let Some(selection) = select_domain(services.hosting, request_stub.header("host"))
            .map_err(|_| ConnectionError::HostingLockPoisoned)?
        else {
            let response = Response::text(421, "Misdirected Request", b"unexpected host\n");
            write_response_with_timeout(&mut stream, services.config, response, false, is_tls)
                .await?;
            break;
        };
        let domain = selection.domain;
        let multi_domain = selection.multi_domain;
        let matched_host = selection.matched_host;
        let _domain_permit = match admit_domain_request(&domain).await {
            Ok(permit) => permit,
            Err(AdmissionError::QueueFull) => {
                let response =
                    Response::text(503, "Service Unavailable", b"domain request queue full\n");
                write_response_with_timeout(&mut stream, &domain.config, response, false, is_tls)
                    .await?;
                break;
            }
            Err(AdmissionError::QueueTimeout | AdmissionError::Closed) => {
                let response = Response::text(
                    503,
                    "Service Unavailable",
                    b"domain request queue timeout\n",
                );
                write_response_with_timeout(&mut stream, &domain.config, response, false, is_tls)
                    .await?;
                break;
            }
        };
        let program = domain.program.as_ref();
        let config = &domain.config;
        let appfs = domain.appfs.as_deref();
        let static_assets = domain.static_assets.as_deref();
        let resource_profiles = domain.resource_profiles.as_ref();
        let max_image_pixels = domain.max_image_pixels;
        let expected_host = if !multi_domain {
            legacy_expected_host
        } else {
            legacy_expected_host.or(matched_host.as_deref())
        };
        let domain_namespace = domain.host.as_deref().unwrap_or("__default__");
        let request_is_https = match effective_request_https(
            &request_stub,
            peer_ip,
            is_tls,
            &services.web.trusted_proxy_cidrs,
        ) {
            Ok(v) => v,
            Err(_) => {
                let mut response =
                    Response::text(400, "Bad Request", b"invalid forwarding headers\n");
                observe_response(
                    &mut response,
                    &request_id,
                    &observed_method,
                    "__invalid__",
                    &peer_ip.to_string(),
                    observed_bytes_in,
                    &request_timer,
                    services.metrics,
                    services.observability.access_log,
                );
                write_response_with_timeout(&mut stream, config, response, false, is_tls).await?;
                break;
            }
        };
        if let Some(expected) = expected_host {
            if !host_matches_public(request_stub.header("host"), expected) {
                let mut response = Response::text(421, "Misdirected Request", b"unexpected host\n");
                observe_response(
                    &mut response,
                    &request_id,
                    &observed_method,
                    "__invalid__",
                    &peer_ip.to_string(),
                    observed_bytes_in,
                    &request_timer,
                    services.metrics,
                    services.observability.access_log,
                );
                write_response_with_timeout(
                    &mut stream,
                    services.config,
                    response,
                    false,
                    request_is_https,
                )
                .await?;
                break;
            }
        }
        let keep_alive =
            head.keep_alive && request_index + 1 < services.config.max_requests_per_connection;
        let request_origin = head
            .headers
            .iter()
            .find(|(n, _)| n == "origin")
            .map(|(_, v)| v.clone());
        let request_accept = head
            .headers
            .iter()
            .find(|(n, _)| n == "accept")
            .map(|(_, v)| v.clone());
        let early_path = head
            .target
            .split_once('?')
            .map(|v| v.0)
            .unwrap_or(head.target.as_str());
        if let Some(mut early) = dispatch_early_request(
            early_path,
            &head.method,
            keep_alive,
            services.lifecycle,
            services.database,
            services.auth_runtime.redis.as_ref(),
            program,
            domain_namespace,
            appfs,
            &request_stub,
            max_image_pixels,
            static_assets,
            request_origin.as_deref(),
            services.web,
            services.metrics,
        )
        .await
        {
            observe_response(
                &mut early.response,
                &request_id,
                &observed_method,
                early.route_label,
                &peer_ip.to_string(),
                observed_bytes_in,
                &request_timer,
                services.metrics,
                services.observability.access_log,
            );
            write_response_with_timeout(
                &mut stream,
                config,
                early.response,
                early.keep_alive,
                request_is_https,
            )
            .await?;
            if !early.keep_alive {
                break;
            }
            continue;
        }
        let (session, is_new) = resolve_session(
            &request_stub,
            config,
            services.sessions,
            services.auth_runtime,
        )
        .await?;
        let effective_peer =
            match effective_client_ip(&request_stub, peer_ip, &services.web.trusted_proxy_cidrs) {
                Ok(ip) => ip.to_string(),
                Err(_) => {
                    let mut response =
                        Response::text(400, "Bad Request", b"invalid forwarding headers\n");
                    observe_response(
                        &mut response,
                        &request_id,
                        &observed_method,
                        "__invalid__",
                        &peer_ip.to_string(),
                        observed_bytes_in,
                        &request_timer,
                        services.metrics,
                        services.observability.access_log,
                    );
                    write_response_with_timeout(
                        &mut stream,
                        config,
                        response,
                        false,
                        request_is_https,
                    )
                    .await?;
                    break;
                }
            };
        let method = HttpMethod::parse(&head.method);
        let path = head
            .target
            .split_once('?')
            .map(|v| v.0.to_string())
            .unwrap_or_else(|| head.target.clone());
        let route_label = format!(
            "{}:{}",
            domain_namespace,
            method
                .and_then(|m| route_meta_for_request(program, m, &path).ok())
                .map(|r| r.name.clone())
                .unwrap_or_else(|| "__unmatched__".into())
        );
        let upload_route = method
            .and_then(|m| route_meta_for_request(program, m, &path).ok())
            .and_then(|r| r.upload.as_ref().map(|u| (r, u)));

        let dispatch_ctx = connection_dispatch::RequestExecutionContext {
            program,
            native_runtime: domain.native_runtime.as_ref(),
            config,
            appfs,
            resource_profiles,
            max_image_pixels,
            request_stub: &request_stub,
            request_accept: request_accept.as_deref(),
            request_id: &request_id,
            request_is_https,
            expected_host,
            effective_peer: &effective_peer,
            domain_namespace,
            session: &session,
            services,
        };
        let dispatch = if let Some((route, upload)) = upload_route {
            connection_dispatch::dispatch_upload(
                &mut stream,
                &mut buffer,
                &head,
                route,
                upload,
                &dispatch_ctx,
            )
            .await
        } else {
            connection_dispatch::dispatch_buffered(&mut stream, &mut buffer, head, &dispatch_ctx)
                .await
        };
        let force_close = dispatch.force_close;
        let response = dispatch.response;
        let response_ctx = connection_finalize::ResponseContext {
            request_origin: request_origin.as_deref(),
            request_id: &request_id,
            observed_method: &observed_method,
            route_label: &route_label,
            effective_peer: &effective_peer,
            observed_bytes_in,
            request_timer: &request_timer,
            session: &session,
            is_new_session: is_new,
            config,
            metrics: services.metrics,
            observability: services.observability,
            web: services.web,
        };
        let response = connection_finalize::finalize_response(response, &response_ctx);
        let final_keep_alive = keep_alive && upload_route.is_none() && !force_close;
        write_response_with_timeout(
            &mut stream,
            config,
            response,
            final_keep_alive,
            request_is_https,
        )
        .await?;
        if !final_keep_alive {
            break;
        }
    }
    let _ = stream.shutdown().await;
    Ok(())
}
