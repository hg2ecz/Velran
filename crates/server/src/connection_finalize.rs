use crate::http_io::Response;
use crate::observe_response;
use crate::response_headers::HeaderName;
use crate::session_cookie::render as session_cookie;
use crate::web_security::apply_cors_headers;
use crate::{ObservabilityCliConfig, WebSecurityCliConfig};
use auth::SessionSnapshot;
use language_core::ServerConfig;
use observability::{ActivityEvent, Metrics, RequestTimer, audit_log, json_line, utc_timestamp};

pub(super) struct ResponseContext<'a> {
    pub(super) request_origin: Option<&'a str>,
    pub(super) request_id: &'a str,
    pub(super) observed_method: &'a str,
    pub(super) route_label: &'a str,
    pub(super) effective_peer: &'a str,
    pub(super) observed_bytes_in: u64,
    pub(super) request_timer: &'a RequestTimer,
    pub(super) session: &'a SessionSnapshot,
    pub(super) is_new_session: bool,
    pub(super) config: &'a ServerConfig,
    pub(super) metrics: &'a Metrics,
    pub(super) observability: &'a ObservabilityCliConfig,
    pub(super) web: &'a WebSecurityCliConfig,
}

pub(super) fn finalize_response(mut response: Response, ctx: &ResponseContext<'_>) -> Response {
    apply_cors_headers(&mut response, ctx.request_origin, ctx.web);
    attach_session_cookie(&mut response, ctx);
    observe_response(
        &mut response,
        ctx.request_id,
        ctx.observed_method,
        ctx.route_label,
        ctx.effective_peer,
        ctx.observed_bytes_in,
        ctx.request_timer,
        ctx.metrics,
        ctx.observability.access_log,
    );
    audit_user_activity(&response, ctx);
    response
}

fn attach_session_cookie(response: &mut Response, ctx: &ResponseContext<'_>) {
    if !ctx.is_new_session || response.has_header(HeaderName::SetCookie) {
        return;
    }
    response.push_header(
        HeaderName::SetCookie,
        session_cookie(ctx.config, &ctx.session.id, ctx.web.cors_allow_credentials),
    );
}

fn audit_user_activity(response: &Response, ctx: &ResponseContext<'_>) {
    if ctx.route_label.starts_with("__")
        || (ctx.observed_method != "POST" && response.status != 403)
    {
        return;
    }
    let Some(actor) = ctx.session.principal.as_deref() else {
        return;
    };
    let outcome = if response.status < 400 {
        "success"
    } else if response.status < 500 {
        "denied"
    } else {
        "error"
    };
    if let Ok(line) = json_line(&ActivityEvent {
        schema_version: 1,
        timestamp: utc_timestamp(),
        event: "user_activity",
        request_id: ctx.request_id,
        actor,
        action: ctx.route_label,
        target: "application_route",
        outcome,
        client_ip: ctx.effective_peer,
    }) {
        audit_log(&line);
    }
}
