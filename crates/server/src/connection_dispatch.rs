use crate::check_route_rate_limit;
use crate::connection::ConnectionServices;
use crate::http_dispatch;
use crate::http_io::{HttpRequest, ParsedHead, Response, read_buffered_body};
use crate::presentation::{
    accepts_media, app_error_response, authorize_route, read_error_response,
};
use crate::request_input::{encode_image_descriptor, encode_upload_descriptor};
use crate::response_kind::route_returns_json;
use crate::web_security::validate_browser_state_change;
use auth::SessionSnapshot;
use language_core::{Program, Route, ServerConfig, UploadField};
use runtime::{AppResponse, NativeRequest, NativeRequestValue, ResourceProfiles};
use std::io::Cursor;
use std::time::Duration;
use storage::{AppFs, UploadError, multipart_boundary, store_single_multipart_file};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tokio::time::timeout;

pub(super) struct DispatchOutcome {
    pub(super) response: Response,
    pub(super) force_close: bool,
}

impl DispatchOutcome {
    fn keep_connection(response: Response) -> Self {
        Self {
            response,
            force_close: false,
        }
    }

    fn close_connection(response: Response) -> Self {
        Self {
            response,
            force_close: true,
        }
    }
}

pub(super) struct RequestExecutionContext<'a, 's> {
    pub(super) program: &'a Program,
    pub(super) native_runtime: Option<&'a crate::native_runtime::SharedNativeRuntime>,
    pub(super) config: &'a ServerConfig,
    pub(super) appfs: Option<&'a AppFs>,
    pub(super) resource_profiles: &'a ResourceProfiles,
    pub(super) max_image_pixels: u64,
    pub(super) request_stub: &'a HttpRequest,
    pub(super) request_accept: Option<&'a str>,
    pub(super) request_id: &'a str,
    pub(super) request_is_https: bool,
    pub(super) expected_host: Option<&'a str>,
    pub(super) effective_peer: &'a str,
    pub(super) domain_namespace: &'a str,
    pub(super) session: &'a SessionSnapshot,
    pub(super) services: &'a ConnectionServices<'s>,
}

pub(super) async fn dispatch_upload<S>(
    stream: &mut S,
    buffer: &mut Vec<u8>,
    head: &ParsedHead,
    route: &Route,
    upload: &UploadField,
    ctx: &RequestExecutionContext<'_, '_>,
) -> DispatchOutcome
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    let json_api = route_returns_json(ctx.program, route);
    let Some(fs) = ctx.appfs else {
        return DispatchOutcome::keep_connection(Response::text(
            503,
            "Service Unavailable",
            b"upload storage unavailable\n",
        ));
    };
    if head.content_length as u64 > fs.limits().max_file_bytes.saturating_add(64 * 1024) {
        return DispatchOutcome::keep_connection(Response::text(
            413,
            "Content Too Large",
            b"upload too large\n",
        ));
    }
    if !ctx.request_is_https && !ctx.config.insecure_dev_cookies {
        return DispatchOutcome::keep_connection(Response::text(
            426,
            "Upgrade Required",
            b"HTTPS required\n",
        ));
    }
    if let Err(response) = validate_browser_state_change(
        ctx.request_stub,
        ctx.request_is_https,
        ctx.expected_host,
        ctx.services.web,
    ) {
        return DispatchOutcome::keep_connection(response);
    }
    if let Some(response) = authorize_route(&route.auth, ctx.session, json_api) {
        return DispatchOutcome::keep_connection(response);
    }
    if let Some(response) = check_route_rate_limit(
        ctx.services.route_rate_limiter,
        route,
        ctx.effective_peer,
        ctx.session.principal.as_deref(),
        json_api,
    )
    .await
    {
        return DispatchOutcome::keep_connection(response);
    }
    let boundary = match multipart_boundary(ctx.request_stub.header("content-type").unwrap_or("")) {
        Ok(v) => v,
        Err(_) => {
            return DispatchOutcome::close_connection(Response::text(
                415,
                "Unsupported Media Type",
                b"expected multipart/form-data\n",
            ));
        }
    };
    if route.multipart_schema.is_some()
        && route.multipart_fields.len().saturating_sub(1) > ctx.config.max_form_fields
    {
        return DispatchOutcome::keep_connection(Response::text(
            413,
            "Content Too Large",
            b"too many multipart fields\n",
        ));
    }
    let destination = match fs.random_upload_destination(&upload.destination) {
        Ok(v) => v,
        Err(_) => {
            return DispatchOutcome::close_connection(Response::text(
                500,
                "Internal Server Error",
                b"upload destination error\n",
            ));
        }
    };
    let initial_len = buffer.len().min(head.content_length);
    let initial: Vec<u8> = buffer.drain(..initial_len).collect();
    let remaining = head.content_length - initial_len;
    let reader = Cursor::new(initial).chain((&mut *stream).take(remaining as u64));
    let multipart_text_names: Vec<String> = route
        .multipart_fields
        .iter()
        .filter(|f| {
            !matches!(
                f.ty,
                language_core::ValueType::Upload | language_core::ValueType::Image
            )
        })
        .map(|f| f.name.clone())
        .collect();
    let stored = timeout(
        Duration::from_millis(ctx.config.request_timeout_ms),
        store_single_multipart_file(
            reader,
            &boundary,
            fs,
            &destination,
            head.content_length as u64,
            &ctx.session.csrf_token,
            if route.multipart_schema.is_some() {
                &upload.name
            } else {
                "file"
            },
            &multipart_text_names,
            ctx.config.max_form_field_bytes as u64,
        ),
    )
    .await;
    let info = match stored {
        Err(_) => {
            return DispatchOutcome::close_connection(Response::text(
                503,
                "Service Unavailable",
                b"upload timeout\n",
            ));
        }
        Ok(Err(UploadError::Fs(storage::FsError::FileTooLarge))) => {
            return DispatchOutcome::keep_connection(Response::text(
                413,
                "Content Too Large",
                b"upload too large\n",
            ));
        }
        Ok(Err(_)) => {
            return DispatchOutcome::close_connection(Response::text(
                400,
                "Bad Request",
                b"invalid multipart upload\n",
            ));
        }
        Ok(Ok(v)) => v,
    };
    let descriptor = if upload.image {
        match encode_image_descriptor(fs, &destination, &info, ctx.max_image_pixels).await {
            Ok(v) => NativeRequestValue::Image(v),
            Err(_) => {
                fs.cleanup_upload(&info);
                return DispatchOutcome::close_connection(Response::text(
                    415,
                    "Unsupported Media Type",
                    b"unsupported or invalid image\n",
                ));
            }
        }
    } else {
        NativeRequestValue::Upload(encode_upload_descriptor(&destination, &info))
    };
    let Some(native_runtime) = ctx.native_runtime else {
        fs.cleanup_upload(&info);
        return DispatchOutcome::close_connection(Response::text(
            503,
            "Service Unavailable",
            b"native runtime unavailable\n",
        ));
    };
    let request = if route.multipart_schema.is_some() {
        let request_path = ctx
            .request_stub
            .target
            .split_once('?')
            .map(|v| v.0)
            .unwrap_or(ctx.request_stub.target.as_str());
        match runtime::prepare_native_multipart_request_for_route(
            ctx.program,
            route,
            request_path,
            &info.text_fields,
            &upload.name,
            descriptor,
        ) {
            Ok(Some(request)) => request,
            Ok(None) | Err(_) => {
                fs.cleanup_upload(&info);
                return DispatchOutcome::close_connection(Response::text(
                    400,
                    "Bad Request",
                    b"invalid multipart fields\n",
                ));
            }
        }
    } else {
        NativeRequest {
            handler_name: route.handler.clone(),
            values: vec![descriptor],
        }
    };
    let execution = crate::app_execution::execute_prepared(
        native_runtime,
        request,
        ctx.config,
        ctx.services.metrics,
    );
    let success = execution.is_ok();
    if success {
        if fs.commit_upload(&info, &destination).is_err() {
            fs.cleanup_upload(&info);
            return DispatchOutcome::close_connection(Response::text(
                503,
                "Service Unavailable",
                b"upload commit failed\n",
            ));
        }
    } else {
        fs.cleanup_upload(&info);
    }
    let response = match execution {
        Ok(AppResponse::Html(html)) if accepts_media(ctx.request_accept, "text/html") => {
            Response::new(
                200,
                "OK",
                "text/html; charset=utf-8",
                html.as_str().as_bytes(),
            )
        }
        Ok(AppResponse::Json(json)) if accepts_media(ctx.request_accept, "application/json") => {
            Response::new(
                200,
                "OK",
                "application/json; charset=utf-8",
                json.as_bytes(),
            )
        }
        Ok(AppResponse::Html(_)) => {
            Response::text(406, "Not Acceptable", b"text/html is not acceptable\n")
        }
        Ok(AppResponse::Json(_)) => Response::text(
            406,
            "Not Acceptable",
            b"application/json is not acceptable\n",
        ),
        Ok(AppResponse::Redirect(redirect)) => Response::redirect(
            redirect.status().code(),
            redirect.status().reason(),
            redirect.location(),
        ),
        Err(error) => app_error_response(error, json_api),
    };
    DispatchOutcome::keep_connection(response)
}

pub(super) async fn dispatch_buffered<S>(
    stream: &mut S,
    buffer: &mut Vec<u8>,
    head: ParsedHead,
    ctx: &RequestExecutionContext<'_, '_>,
) -> DispatchOutcome
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    if head.content_length > ctx.config.max_body_bytes {
        return DispatchOutcome::keep_connection(Response::text(
            413,
            "Content Too Large",
            b"request body too large\n",
        ));
    }
    let body = match timeout(
        Duration::from_millis(ctx.config.read_timeout_ms),
        read_buffered_body(stream, buffer, head.content_length),
    )
    .await
    {
        Err(_) => {
            return DispatchOutcome::close_connection(Response::text(
                408,
                "Request Timeout",
                b"request timeout\n",
            ));
        }
        Ok(Ok(body)) => body,
        Ok(Err(error)) => {
            return DispatchOutcome::close_connection(read_error_response(error));
        }
    };
    let request = HttpRequest {
        method: head.method,
        target: head.target,
        headers: head.headers,
        body,
    };
    let response = match timeout(
        Duration::from_millis(ctx.config.request_timeout_ms),
        http_dispatch::dispatch(
            ctx.program,
            ctx.native_runtime,
            &request,
            ctx.config,
            ctx.services.sessions,
            ctx.session,
            ctx.services.database,
            ctx.services.auth_runtime,
            ctx.resource_profiles,
            ctx.services.route_rate_limiter,
            ctx.services.public_cache,
            ctx.services.idempotency_redis,
            ctx.services.outbound,
            ctx.services.metrics,
            ctx.request_id,
            ctx.effective_peer,
            ctx.domain_namespace,
            ctx.request_is_https,
            ctx.expected_host,
            ctx.services.web,
        ),
    )
    .await
    {
        Ok(response) => response,
        Err(_) => Response::text(503, "Service Unavailable", b"request execution timeout\n"),
    };
    DispatchOutcome::keep_connection(response)
}
