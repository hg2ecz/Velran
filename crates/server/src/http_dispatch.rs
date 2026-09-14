use crate::auth_http::{auth_login, auth_logout};
use crate::bootstrap_config::{CachedPage, PublicPageCache};
use crate::http_io::{HttpRequest, Response};
use crate::presentation::{
    accepts_media, authorize_route, conflict_response, endpoint_error, render_form_failure,
};
use crate::rate_limit::RouteRateLimiter;
use crate::request_input::media_type_is;
use crate::request_json::decode_route_json;
use crate::response_headers::HeaderName;
use crate::response_kind::route_returns_json;
use crate::web_security::{cors_preflight, validate_browser_state_change};
use crate::{AuthRuntime, WebSecurityCliConfig, public_cache_key};
use auth::{SessionBackend, SessionSnapshot};
use data::{Database, RedisStore};
use language_core::{AppError, HttpMethod, ServerConfig};
use observability::{Metrics, server_log};
use runtime::{AppResponse, ResourceProfiles, decode_urlencoded_limited, route_meta_for_request};
use std::time::Duration;
use tokio::time::timeout;
pub(super) async fn dispatch(
    program: &language_core::Program,
    native_runtime: Option<&crate::native_runtime::SharedNativeRuntime>,
    request: &HttpRequest,
    config: &ServerConfig,
    sessions: &SessionBackend,
    session: &SessionSnapshot,
    database: Option<&Database>,
    auth_runtime: &AuthRuntime,
    resource_profiles: &ResourceProfiles,
    route_rate_limiter: &RouteRateLimiter,
    public_cache: &PublicPageCache,
    idempotency_redis: Option<&RedisStore>,
    outbound: Option<&dyn runtime::OutboundRuntime>,
    metrics: &Metrics,
    request_id: &str,
    peer_key: &str,
    domain_namespace: &str,
    is_tls: bool,
    expected_host: Option<&str>,
    web: &WebSecurityCliConfig,
) -> Response {
    if !is_tls && !config.insecure_dev_cookies {
        return Response::text(426, "Upgrade Required", b"HTTPS required\n");
    }
    if request.method == "OPTIONS" {
        if request.header("origin").is_some()
            || request.header("access-control-request-method").is_some()
        {
            return cors_preflight(request, web, program);
        }
        return Response::text(405, "Method Not Allowed", b"method not allowed\n");
    }
    let method = match HttpMethod::parse(&request.method) {
        Some(v) => v,
        None => return Response::text(405, "Method Not Allowed", b"method not allowed\n"),
    };
    let (path, raw_query) = request
        .target
        .split_once('?')
        .unwrap_or((request.target.as_str(), ""));
    if path == "/__velran/auth/login" {
        if let Err(status) = validate_browser_state_change(request, is_tls, expected_host, web) {
            return status;
        }
        return auth_login(
            request,
            config,
            sessions,
            session,
            auth_runtime,
            request_id,
            peer_key,
            web,
        )
        .await;
    }
    if path == "/__velran/auth/logout" {
        if let Err(status) = validate_browser_state_change(request, is_tls, expected_host, web) {
            return status;
        }
        return auth_logout(
            request, config, sessions, session, request_id, peer_key, web,
        )
        .await;
    }
    let route = match route_meta_for_request(program, method, path) {
        Ok(r) => r,
        Err(AppError::BadRequest) => return Response::text(400, "Bad Request", b"bad request\n"),
        Err(AppError::MethodNotAllowed) => {
            return Response::text(405, "Method Not Allowed", b"method not allowed\n");
        }
        Err(_) => return Response::text(404, "Not Found", b"not found\n"),
    };
    let json_api = route_returns_json(program, route);
    if method == HttpMethod::Post && !matches!(route.auth, language_core::RouteAuth::Webhook(_)) {
        if let Err(status) = validate_browser_state_change(request, is_tls, expected_host, web) {
            return status;
        }
    }
    if let Err(response) = crate::webhook_verification::verify(
        program,
        route,
        request,
        idempotency_redis,
        domain_namespace,
        web,
        json_api,
    )
    .await
    {
        return response;
    }
    if let Some(response) = authorize_route(&route.auth, session, json_api) {
        return response;
    }
    if let Some(policy) = route.rate_policy.as_deref() {
        match route_rate_limiter
            .check(
                policy,
                &format!("{domain_namespace}:{}", route.name),
                peer_key,
                session.principal.as_deref(),
            )
            .await
        {
            Ok((true, _)) => {}
            Ok((false, retry_after)) => {
                let mut r = endpoint_error(
                    json_api,
                    429,
                    "Too Many Requests",
                    "rate_limited",
                    b"rate limit exceeded\n",
                );
                r.push_header(HeaderName::RetryAfter, retry_after.to_string());
                return r;
            }
            Err(_) => {
                return endpoint_error(
                    json_api,
                    503,
                    "Service Unavailable",
                    "rate_limiter_unavailable",
                    b"rate limiter unavailable\n",
                );
            }
        }
    }
    let query_pairs = if raw_query.is_empty() {
        Vec::new()
    } else {
        match decode_urlencoded_limited(
            raw_query.as_bytes(),
            config.max_form_fields,
            config.max_form_field_bytes,
        ) {
            Ok(v) => v,
            Err(_) => {
                return endpoint_error(
                    json_api,
                    400,
                    "Bad Request",
                    "bad_request",
                    b"bad query string\n",
                );
            }
        }
    };
    if method == HttpMethod::Post && !query_pairs.is_empty() {
        return endpoint_error(
            json_api,
            400,
            "Bad Request",
            "bad_request",
            b"POST query parameters are not supported\n",
        );
    }
    let mut cache_guard: Option<tokio::sync::OwnedSemaphorePermit> = None;
    let cache_key = if method == HttpMethod::Get {
        if let Some(policy) = route.public_cache.as_ref() {
            let media = if json_api {
                "application/json"
            } else {
                "text/html"
            };
            if !accepts_media(request.header("accept"), media) {
                return Response::text(
                    406,
                    "Not Acceptable",
                    if json_api {
                        b"application/json is not acceptable\n"
                    } else {
                        b"text/html is not acceptable\n"
                    },
                );
            }
            let cache_route = format!("{domain_namespace}:{}", route.name);
            let generation = match public_cache.generation(&cache_route).await {
                Ok(v) => v,
                Err(_) => {
                    return endpoint_error(
                        json_api,
                        503,
                        "Service Unavailable",
                        "cache_unavailable",
                        b"cache unavailable\n",
                    );
                }
            };
            let key = public_cache_key(
                domain_namespace,
                route,
                generation,
                path,
                &query_pairs,
                json_api,
            );
            match public_cache.get(&key).await {
                Ok(Some(hit)) => {
                    metrics.inc_cache_hit();
                    let ct = if hit.content_type.starts_with("application/json") {
                        "application/json; charset=utf-8"
                    } else {
                        "text/html; charset=utf-8"
                    };
                    let mut r = Response::new(200, "OK", ct, &hit.body);
                    r.push_header(
                        HeaderName::CacheControl,
                        format!("public, max-age={}", policy.ttl_secs),
                    );
                    return r;
                }
                Ok(None) => {
                    if public_cache.prune_rebuild_locks().is_err() {
                        return endpoint_error(
                            json_api,
                            503,
                            "Service Unavailable",
                            "cache_unavailable",
                            b"cache unavailable\n",
                        );
                    }
                    let lock = match public_cache.rebuild_lock(&key) {
                        Ok(v) => v,
                        Err(_) => {
                            return endpoint_error(
                                json_api,
                                503,
                                "Service Unavailable",
                                "cache_unavailable",
                                b"cache unavailable\n",
                            );
                        }
                    };
                    let permit = match timeout(
                        Duration::from_millis(public_cache.singleflight_wait_timeout_ms()),
                        lock.acquire_owned(),
                    )
                    .await
                    {
                        Ok(Ok(v)) => v,
                        Ok(Err(_)) => {
                            return endpoint_error(
                                json_api,
                                503,
                                "Service Unavailable",
                                "cache_unavailable",
                                b"cache unavailable\n",
                            );
                        }
                        Err(_) => {
                            return endpoint_error(
                                json_api,
                                503,
                                "Service Unavailable",
                                "cache_fill_timeout",
                                b"cache fill wait timeout\n",
                            );
                        }
                    };
                    match public_cache.get(&key).await {
                        Ok(Some(hit)) => {
                            metrics.inc_cache_hit();
                            let ct = if hit.content_type.starts_with("application/json") {
                                "application/json; charset=utf-8"
                            } else {
                                "text/html; charset=utf-8"
                            };
                            let mut r = Response::new(200, "OK", ct, &hit.body);
                            r.push_header(
                                HeaderName::CacheControl,
                                format!("public, max-age={}", policy.ttl_secs),
                            );
                            return r;
                        }
                        Ok(None) => {
                            metrics.inc_cache_miss();
                            cache_guard = Some(permit);
                            Some(key)
                        }
                        Err(_) => {
                            return endpoint_error(
                                json_api,
                                503,
                                "Service Unavailable",
                                "cache_unavailable",
                                b"cache unavailable\n",
                            );
                        }
                    }
                }
                Err(_) => {
                    return endpoint_error(
                        json_api,
                        503,
                        "Service Unavailable",
                        "cache_unavailable",
                        b"cache unavailable\n",
                    );
                }
            }
        } else {
            None
        }
    } else {
        None
    };
    let form_pairs = if method == HttpMethod::Post {
        if !route.json_fields.is_empty() {
            let content_type = match request.header("content-type") {
                Some(v) => v,
                None => {
                    return endpoint_error(
                        json_api,
                        415,
                        "Unsupported Media Type",
                        "unsupported_media_type",
                        b"expected application/json\n",
                    );
                }
            };
            if !media_type_is(content_type, "application/json") {
                return endpoint_error(
                    json_api,
                    415,
                    "Unsupported Media Type",
                    "unsupported_media_type",
                    b"expected application/json\n",
                );
            }
            if !matches!(route.auth, language_core::RouteAuth::Webhook(_)) {
                let supplied = match request.header("x-csrf-token") {
                    Some(v) => v,
                    None => {
                        return endpoint_error(
                            json_api,
                            403,
                            "Forbidden",
                            "csrf_failed",
                            b"CSRF validation failed\n",
                        );
                    }
                };
                if !matches!(sessions.verify_csrf(&session.id, supplied).await, Ok(true)) {
                    return endpoint_error(
                        json_api,
                        403,
                        "Forbidden",
                        "csrf_failed",
                        b"CSRF validation failed\n",
                    );
                }
            }
            match decode_route_json(&request.body, config) {
                Ok(v) => v,
                Err(_) => {
                    return endpoint_error(
                        json_api,
                        400,
                        "Bad Request",
                        "bad_request",
                        b"bad JSON body\n",
                    );
                }
            }
        } else {
            let content_type = match request.header("content-type") {
                Some(v) => v,
                None => {
                    return Response::text(
                        415,
                        "Unsupported Media Type",
                        b"expected application/x-www-form-urlencoded\n",
                    );
                }
            };
            if !media_type_is(content_type, "application/x-www-form-urlencoded") {
                return Response::text(
                    415,
                    "Unsupported Media Type",
                    b"expected application/x-www-form-urlencoded\n",
                );
            }
            let mut pairs = match decode_urlencoded_limited(
                &request.body,
                config.max_form_fields,
                config.max_form_field_bytes,
            ) {
                Ok(v) => v,
                Err(_) => return Response::text(400, "Bad Request", b"bad form body\n"),
            };
            let pos: Vec<usize> = pairs
                .iter()
                .enumerate()
                .filter_map(|(i, (n, _))| (n == "_csrf").then_some(i))
                .collect();
            if pos.len() != 1 {
                return Response::text(403, "Forbidden", b"CSRF validation failed\n");
            }
            let supplied = pairs[pos[0]].1.clone();
            if !matches!(sessions.verify_csrf(&session.id, &supplied).await, Ok(true)) {
                return Response::text(403, "Forbidden", b"CSRF validation failed\n");
            }
            pairs.remove(pos[0]);
            pairs
        }
    } else {
        Vec::new()
    };
    let idempotency_claim = match crate::idempotency::begin(
        idempotency_redis,
        request,
        route,
        session.principal.as_deref(),
        domain_namespace,
        json_api,
    )
    .await
    {
        Ok(crate::idempotency::BeginResult::Disabled) => None,
        Ok(crate::idempotency::BeginResult::Claimed(claim)) => Some(claim),
        Ok(crate::idempotency::BeginResult::Replay(response)) => return response,
        Err(response) => return response,
    };
    let flash = if method == HttpMethod::Get {
        match sessions.take_flash(&session.id).await {
            Ok(v) => v,
            Err(_) => {
                server_log("{\"event\":\"flash_take_failed\"}");
                None
            }
        }
    } else {
        None
    };
    let had_flash = flash.is_some();
    let mut system_values = crate::http_system_values::base_system_values(session, request_id);
    crate::http_system_values::append_flash(&mut system_values, flash);
    let execution = crate::app_execution::execute(
        program,
        native_runtime,
        route,
        method,
        path,
        &query_pairs,
        &form_pairs,
        config,
        resource_profiles,
        &system_values,
        database,
        outbound,
        metrics,
    )
    .await;
    if let Err(err) = &execution {
        crate::runtime_error_logging::log_runtime_error(
            err,
            request_id,
            domain_namespace,
            &route.name,
            &request.method,
            path,
        );
    }
    let mut response = match execution {
        Ok(AppResponse::Html(html)) => {
            if !accepts_media(request.header("accept"), "text/html") {
                Response::text(406, "Not Acceptable", b"text/html is not acceptable\n")
            } else {
                Response::new(
                    200,
                    "OK",
                    "text/html; charset=utf-8",
                    html.as_str().as_bytes(),
                )
            }
        }
        Ok(AppResponse::Json(json)) => {
            if !accepts_media(request.header("accept"), "application/json") {
                Response::text(
                    406,
                    "Not Acceptable",
                    b"application/json is not acceptable\n",
                )
            } else {
                Response::new(
                    200,
                    "OK",
                    "application/json; charset=utf-8",
                    json.as_bytes(),
                )
            }
        }
        Ok(AppResponse::Redirect(r)) => {
            if let Some(flash) = r.flash() {
                if sessions
                    .set_flash(&session.id, flash.kind.as_str(), &flash.message)
                    .await
                    .is_err()
                {
                    server_log("{\"event\":\"flash_store_failed\"}");
                }
            }
            Response::redirect(r.status().code(), r.status().reason(), r.location())
        }
        Err(AppError::BadRequest) => endpoint_error(
            json_api,
            400,
            "Bad Request",
            "bad_request",
            b"bad request\n",
        ),
        Err(AppError::FormInvalid(failure)) => {
            render_form_failure(program, route, &failure, path, &session.csrf_token)
        }
        Err(AppError::UnsupportedMediaType) => endpoint_error(
            json_api,
            415,
            "Unsupported Media Type",
            "unsupported_media_type",
            b"unsupported media type\n",
        ),
        Err(AppError::NotFound) => {
            endpoint_error(json_api, 404, "Not Found", "not_found", b"not found\n")
        }
        Err(AppError::Forbidden) => {
            endpoint_error(json_api, 403, "Forbidden", "forbidden", b"forbidden\n")
        }
        Err(AppError::Conflict) => conflict_response(json_api),
        Err(AppError::MethodNotAllowed) => endpoint_error(
            json_api,
            405,
            "Method Not Allowed",
            "method_not_allowed",
            b"method not allowed\n",
        ),
        Err(AppError::InstructionLimit) => endpoint_error(
            json_api,
            503,
            "Service Unavailable",
            "resource_limit",
            b"request execution limit exceeded\n",
        ),
        Err(AppError::MemoryLimit) => endpoint_error(
            json_api,
            503,
            "Service Unavailable",
            "resource_limit",
            b"request memory limit exceeded\n",
        ),
        Err(AppError::DeadlineExceeded) => endpoint_error(
            json_api,
            503,
            "Service Unavailable",
            "resource_limit",
            b"request execution deadline exceeded\n",
        ),
        Err(AppError::ExternalIoLimit) => endpoint_error(
            json_api,
            503,
            "Service Unavailable",
            "resource_limit",
            b"request external I/O limit exceeded\n",
        ),
        Err(AppError::Database) => endpoint_error(
            json_api,
            503,
            "Service Unavailable",
            "database_unavailable",
            b"database operation failed\n",
        ),
        Err(AppError::Internal) => endpoint_error(
            json_api,
            500,
            "Internal Server Error",
            "internal_error",
            b"internal server error\n",
        ),
    };
    if had_flash {
        response.remove_header(HeaderName::CacheControl);
        response.push_header(HeaderName::CacheControl, "no-store");
    }
    if let (Some(key), Some(policy)) = (cache_key.as_deref(), route.public_cache.as_ref()) {
        if response.status == 200 {
            let cached = CachedPage {
                content_type: response.content_type().to_string(),
                body: response.body.clone(),
            };
            if public_cache
                .set(key, cached, policy.ttl_secs)
                .await
                .is_err()
            {
                return endpoint_error(
                    json_api,
                    503,
                    "Service Unavailable",
                    "cache_unavailable",
                    b"cache unavailable\n",
                );
            }
            response.push_header(
                HeaderName::CacheControl,
                format!("public, max-age={}", policy.ttl_secs),
            );
        }
    }
    drop(cache_guard);
    if method == HttpMethod::Post && (200..400).contains(&response.status) {
        for target in &route.invalidate_caches {
            let cache_target = format!("{domain_namespace}:{target}");
            if public_cache.invalidate_route(&cache_target).await.is_err() {
                return endpoint_error(
                    json_api,
                    503,
                    "Service Unavailable",
                    "cache_unavailable",
                    b"cache invalidation failed\n",
                );
            }
        }
    }
    if let Some(claim) = idempotency_claim {
        crate::idempotency::complete(idempotency_redis, claim, &response).await;
    }
    response
}
