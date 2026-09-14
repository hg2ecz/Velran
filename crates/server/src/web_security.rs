use crate::http_io::HttpRequest;
use crate::response_headers::HeaderName;
use crate::{Response, WebSecurityCliConfig};
use ipnet::IpNet;
use language_core::{AppError, HttpMethod, Program};
use runtime::route_meta_for_request;
use std::net::IpAddr;

pub(super) fn effective_client_ip(
    request: &HttpRequest,
    peer_ip: IpAddr,
    trusted: &[IpNet],
) -> Result<IpAddr, ()> {
    let forwarded = request.header("forwarded");
    let xff = request.header("x-forwarded-for");
    let xri = request.header("x-real-ip");
    let supplied =
        usize::from(forwarded.is_some()) + usize::from(xff.is_some()) + usize::from(xri.is_some());
    if supplied > 1 {
        return Err(());
    }
    if supplied == 0 {
        return Ok(peer_ip);
    }
    if !trusted.iter().any(|net| net.contains(&peer_ip)) {
        return Err(());
    }
    if let Some(value) = xff.or(xri) {
        if value.contains(',') {
            return Err(());
        }
        return value.trim().parse::<IpAddr>().map_err(|_| ());
    }
    let Some(value) = forwarded else {
        return Err(());
    };
    if value.contains(',') {
        return Err(());
    }
    for element in value.split(',') {
        for param in element.split(';') {
            let (name, val) = param.trim().split_once('=').ok_or(())?;
            if name.eq_ignore_ascii_case("for") {
                let raw = val.trim().trim_matches('"');
                let raw = raw
                    .strip_prefix('[')
                    .and_then(|v| v.strip_suffix(']'))
                    .unwrap_or(raw);
                return raw.parse::<IpAddr>().map_err(|_| ());
            }
        }
    }
    Err(())
}

pub(super) fn effective_request_https(
    request: &HttpRequest,
    peer_ip: IpAddr,
    transport_tls: bool,
    trusted: &[IpNet],
) -> Result<bool, ()> {
    if transport_tls {
        return Ok(true);
    }
    let forwarded = request.header("forwarded");
    let xfp = request.header("x-forwarded-proto");
    if forwarded.is_some() && xfp.is_some() {
        return Err(());
    }
    if forwarded.is_none() && xfp.is_none() {
        return Ok(false);
    }
    if !trusted.iter().any(|net| net.contains(&peer_ip)) {
        return Err(());
    }
    if let Some(value) = xfp {
        if value.contains(',') {
            return Err(());
        }
        return match value.trim().to_ascii_lowercase().as_str() {
            "https" => Ok(true),
            "http" => Ok(false),
            _ => Err(()),
        };
    }
    let Some(value) = forwarded else {
        return Err(());
    };
    if value.contains(',') {
        return Err(());
    }
    let mut proto = None;
    for param in value.split(';') {
        let Some((name, val)) = param.trim().split_once('=') else {
            return Err(());
        };
        if name.eq_ignore_ascii_case("proto") {
            if proto.is_some() {
                return Err(());
            }
            proto = Some(val.trim().trim_matches('"').to_ascii_lowercase());
        }
    }
    match proto.as_deref() {
        Some("https") => Ok(true),
        Some("http") | None => Ok(false),
        _ => Err(()),
    }
}

pub(super) fn validate_browser_state_change(
    request: &HttpRequest,
    is_tls: bool,
    expected_host: Option<&str>,
    web: &WebSecurityCliConfig,
) -> Result<(), Response> {
    let cors_allowed = request
        .header("origin")
        .map(|o| web.cors_origins.iter().any(|v| v == o))
        .unwrap_or(false);
    if let Some(site) = request.header("sec-fetch-site") {
        if !matches!(site, "same-origin" | "same-site" | "none") && !cors_allowed {
            return Err(Response::text(
                403,
                "Forbidden",
                b"cross-site request rejected\n",
            ));
        }
    }
    if web.allow_missing_origin {
        return Ok(());
    }
    let Some(host) = expected_host else {
        return Ok(());
    };
    let scheme = if is_tls { "https" } else { "http" };
    let expected = format!("{scheme}://{host}");
    if let Some(origin) = request.header("origin") {
        if origin == expected {
            return Ok(());
        }
        if cors_allowed && web.cors_allow_credentials {
            return Ok(());
        }
        return Err(Response::text(
            403,
            "Forbidden",
            b"origin validation failed\n",
        ));
    }
    if let Some(referer) = request.header("referer") {
        let prefix = format!("{expected}/");
        return if referer == expected || referer.starts_with(&prefix) {
            Ok(())
        } else {
            Err(Response::text(
                403,
                "Forbidden",
                b"origin validation failed\n",
            ))
        };
    }
    Err(Response::text(403, "Forbidden", b"origin required\n"))
}

pub(super) fn valid_cors_origin(origin: &str) -> bool {
    let rest = origin
        .strip_prefix("https://")
        .or_else(|| origin.strip_prefix("http://"));
    matches!(rest,Some(v) if !v.is_empty()&&!v.contains('/')&&!v.contains('?')&&!v.contains('#')&&!v.contains('@')&&!v.chars().any(|c|c.is_control()||c.is_whitespace()))
}

pub(super) fn cors_preflight(
    request: &HttpRequest,
    web: &WebSecurityCliConfig,
    program: &Program,
) -> Response {
    let Some(origin) = request.header("origin") else {
        return Response::text(400, "Bad Request", b"preflight origin required\n");
    };
    if !web.cors_origins.iter().any(|v| v == origin) {
        return Response::text(403, "Forbidden", b"CORS origin denied\n");
    };
    let Some(method) = request.header("access-control-request-method") else {
        return Response::text(400, "Bad Request", b"preflight method required\n");
    };
    let Some(method) = HttpMethod::parse(method) else {
        return Response::text(405, "Method Not Allowed", b"CORS method denied\n");
    };
    let path = request
        .target
        .split_once('?')
        .map(|v| v.0)
        .unwrap_or(request.target.as_str());
    match route_meta_for_request(program, method, path) {
        Ok(_) => {}
        Err(AppError::MethodNotAllowed) => {
            return Response::text(405, "Method Not Allowed", b"method not allowed\n");
        }
        Err(_) => return Response::text(404, "Not Found", b"not found\n"),
    }
    if let Some(headers) = request.header("access-control-request-headers") {
        for header in headers.split(',').map(|v| v.trim().to_ascii_lowercase()) {
            if header.is_empty()
                || !matches!(header.as_str(), "content-type" | "x-csrf-token" | "accept")
            {
                return Response::text(403, "Forbidden", b"CORS header denied\n");
            }
        }
    }
    let mut response = Response::new(204, "No Content", "text/plain; charset=utf-8", b"");
    response.push_header(HeaderName::AccessControlAllowOrigin, origin);
    response.push_header(HeaderName::AccessControlAllowMethods, "GET, POST, OPTIONS");
    response.push_header(
        HeaderName::AccessControlAllowHeaders,
        "Content-Type, X-CSRF-Token, Accept",
    );
    response.push_header(HeaderName::AccessControlMaxAge, "600");
    response.push_header(HeaderName::Vary, "Origin");
    if web.cors_allow_credentials {
        response.push_header(HeaderName::AccessControlAllowCredentials, "true");
    }
    response
}

pub(super) fn apply_cors_headers(
    response: &mut Response,
    origin: Option<&str>,
    web: &WebSecurityCliConfig,
) {
    if response.has_header(HeaderName::AccessControlAllowOrigin) {
        return;
    }
    let Some(origin) = origin else { return };
    if !web.cors_origins.iter().any(|v| v == origin) {
        return;
    };
    response.push_header(HeaderName::AccessControlAllowOrigin, origin);
    response.push_header(HeaderName::Vary, "Origin");
    if web.cors_allow_credentials {
        response.push_header(HeaderName::AccessControlAllowCredentials, "true");
    }
}
