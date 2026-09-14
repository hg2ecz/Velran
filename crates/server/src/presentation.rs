use crate::http_io::{HttpReadError, Response};
use crate::response_headers::HeaderName;
use auth::SessionSnapshot;
use language_core::{AppError, FormFailure, Program, Route, RouteAuth, ValueType};
use std::collections::HashMap;

pub(super) fn app_error_response(error: AppError, json_api: bool) -> Response {
    match error {
        AppError::BadRequest => endpoint_error(
            json_api,
            400,
            "Bad Request",
            "bad_request",
            b"bad request\n",
        ),
        AppError::FormInvalid(_) => endpoint_error(
            json_api,
            422,
            "Unprocessable Content",
            "validation_failed",
            b"form validation failed\n",
        ),
        AppError::UnsupportedMediaType => endpoint_error(
            json_api,
            415,
            "Unsupported Media Type",
            "unsupported_media_type",
            b"unsupported media type\n",
        ),
        AppError::NotFound => {
            endpoint_error(json_api, 404, "Not Found", "not_found", b"not found\n")
        }
        AppError::Forbidden => {
            endpoint_error(json_api, 403, "Forbidden", "forbidden", b"forbidden\n")
        }
        AppError::Conflict => conflict_response(json_api),
        AppError::MethodNotAllowed => endpoint_error(
            json_api,
            405,
            "Method Not Allowed",
            "method_not_allowed",
            b"method not allowed\n",
        ),
        AppError::InstructionLimit => endpoint_error(
            json_api,
            503,
            "Service Unavailable",
            "resource_limit",
            b"request execution limit exceeded\n",
        ),
        AppError::MemoryLimit => endpoint_error(
            json_api,
            503,
            "Service Unavailable",
            "resource_limit",
            b"request memory limit exceeded\n",
        ),
        AppError::DeadlineExceeded => endpoint_error(
            json_api,
            503,
            "Service Unavailable",
            "resource_limit",
            b"request execution deadline exceeded\n",
        ),
        AppError::ExternalIoLimit => endpoint_error(
            json_api,
            503,
            "Service Unavailable",
            "resource_limit",
            b"request external I/O limit exceeded\n",
        ),
        AppError::Database => endpoint_error(
            json_api,
            503,
            "Service Unavailable",
            "database_unavailable",
            b"database operation failed\n",
        ),
        AppError::Internal => endpoint_error(
            json_api,
            500,
            "Internal Server Error",
            "internal_error",
            b"internal server error\n",
        ),
    }
}

pub(super) fn read_error_response(err: HttpReadError) -> Response {
    match err {
        HttpReadError::HeaderTooLarge => Response::text(
            431,
            "Request Header Fields Too Large",
            b"request headers too large\n",
        ),
        HttpReadError::BodyTooLarge => {
            Response::text(413, "Content Too Large", b"request body too large\n")
        }
        HttpReadError::BadRequest => Response::text(400, "Bad Request", b"bad request\n"),
        HttpReadError::Io => Response::text(400, "Bad Request", b"I/O error\n"),
    }
}

pub(super) fn render_form_failure(
    program: &Program,
    _route: &Route,
    failure: &FormFailure,
    path: &str,
    csrf: &str,
) -> Response {
    let Some(schema) = program.form(&failure.schema) else {
        return Response::text(500, "Internal Server Error", b"form schema missing\n");
    };
    let values: HashMap<&str, &str> = failure
        .values
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let issues: HashMap<&str, &str> = failure
        .issues
        .iter()
        .map(|i| (i.field.as_str(), i.code.as_str()))
        .collect();
    let mut body = String::from(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Validation error</title></head><body><main><h1>Please correct the form</h1><form method=\"post\" action=\"",
    );
    push_html_escaped(&mut body, path);
    body.push_str("\"><input type=\"hidden\" name=\"_csrf\" value=\"");
    push_html_escaped(&mut body, csrf);
    body.push_str("\">");
    for field in &schema.fields {
        let value = values.get(field.name.as_str()).copied().unwrap_or("");
        body.push_str("<div><label>");
        push_html_escaped(&mut body, &field.name);
        body.push(' ');
        let field_type = program.representation_type(field.ty).unwrap_or(field.ty);
        match field_type {
            ValueType::Bool => {
                body.push_str("<select name=\"");
                push_html_escaped(&mut body, &field.name);
                body.push_str("\"><option value=\"false\"");
                if value == "false" {
                    body.push_str(" selected");
                }
                body.push_str(">false</option><option value=\"true\"");
                if value == "true" {
                    body.push_str(" selected");
                }
                body.push_str(">true</option></select>");
            }
            ValueType::Enum(enum_id) => {
                body.push_str("<select name=\"");
                push_html_escaped(&mut body, &field.name);
                body.push_str("\">");
                if let Some(def) = program.enum_by_id(enum_id) {
                    for variant in &def.variants {
                        body.push_str("<option value=\"");
                        push_html_escaped(&mut body, variant);
                        body.push_str("\"");
                        if value == variant {
                            body.push_str(" selected");
                        }
                        body.push('>');
                        push_html_escaped(&mut body, variant);
                        body.push_str("</option>");
                    }
                }
                body.push_str("</select>");
            }
            _ => {
                let input_type = match field_type {
                    ValueType::Int => "number",
                    ValueType::Date => "date",
                    ValueType::Email => "email",
                    ValueType::Url => "url",
                    _ => "text",
                };
                body.push_str("<input type=\"");
                body.push_str(input_type);
                body.push_str("\" name=\"");
                push_html_escaped(&mut body, &field.name);
                body.push_str("\" value=\"");
                push_html_escaped(&mut body, value);
                body.push_str("\">");
            }
        }
        body.push_str("</label>");
        if let Some(code) = issues.get(field.name.as_str()) {
            body.push_str("<p role=\"alert\">Invalid ");
            push_html_escaped(&mut body, &field.name);
            body.push_str(": ");
            push_html_escaped(&mut body, code);
            body.push_str("</p>");
        }
        body.push_str("</div>");
    }
    body.push_str("<button type=\"submit\">Submit</button></form></main></body></html>");
    let mut r = Response::new(
        422,
        "Unprocessable Content",
        "text/html; charset=utf-8",
        body.as_bytes(),
    );
    r.push_header(HeaderName::CacheControl, "no-store");
    r
}
fn push_html_escaped(out: &mut String, value: &str) {
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
}

pub(super) fn authorize_route(
    policy: &RouteAuth,
    session: &SessionSnapshot,
    json_api: bool,
) -> Option<Response> {
    match policy {
        RouteAuth::Public | RouteAuth::Webhook(_) => None,
        RouteAuth::User if session.is_authenticated() => None,
        RouteAuth::Mfa if session.is_authenticated() && session.mfa_verified => None,
        RouteAuth::Role(role) if session.is_authenticated() && session.has_role(role) => None,
        RouteAuth::Permission { roles, .. }
            if session.is_authenticated() && roles.iter().any(|role| session.has_role(role)) =>
        {
            None
        }
        RouteAuth::PermissionMfa { roles, .. }
            if session.is_authenticated()
                && session.mfa_verified
                && roles.iter().any(|role| session.has_role(role)) =>
        {
            None
        }
        RouteAuth::User
        | RouteAuth::Mfa
        | RouteAuth::Role(_)
        | RouteAuth::Permission { .. }
        | RouteAuth::PermissionMfa { .. }
            if !session.is_authenticated() =>
        {
            if json_api {
                Some(endpoint_error(
                    true,
                    401,
                    "Unauthorized",
                    "unauthorized",
                    b"authentication required\n",
                ))
            } else {
                Some(Response::redirect(303, "See Other", "/__velran/auth/login"))
            }
        }
        _ => Some(endpoint_error(
            json_api,
            403,
            "Forbidden",
            "forbidden",
            b"forbidden\n",
        )),
    }
}

pub(super) fn conflict_response(json_api: bool) -> Response {
    if json_api {
        endpoint_error(true, 409, "Conflict", "conflict", b"conflict\n")
    } else {
        let mut response = Response::new(
            409,
            "Conflict",
            "text/html; charset=utf-8",
            b"<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Conflict</title></head><body><main><h1>The data changed</h1><p>Another request changed this record before your save completed. Reload the edit page, review the current values, and submit your changes again.</p></main></body></html>",
        );
        response.push_header(HeaderName::CacheControl, "no-store");
        response
    }
}

pub(super) fn endpoint_error(
    json_api: bool,
    status: u16,
    reason: &'static str,
    code: &'static str,
    text: &'static [u8],
) -> Response {
    if json_api {
        let body = format!(r#"{{"error":"{}"}}"#, code);
        Response::new(
            status,
            reason,
            "application/json; charset=utf-8",
            body.as_bytes(),
        )
    } else {
        Response::text(status, reason, text)
    }
}

pub(super) fn accepts_media(header: Option<&str>, wanted: &str) -> bool {
    let Some(header) = header else { return true };
    let (want_type, want_sub) = wanted.split_once('/').unwrap_or((wanted, ""));
    header.split(',').any(|item| {
        let mut parts = item.trim().split(';');
        let range = parts.next().unwrap_or("").trim();
        let mut quality = 1.0f32;
        for parameter in parts {
            if let Some(raw) = parameter.trim().strip_prefix("q=") {
                quality = match raw.trim().parse::<f32>() {
                    Ok(v) if (0.0..=1.0).contains(&v) => v,
                    _ => return false,
                };
            }
        }
        if quality <= 0.0 {
            return false;
        };
        if range == "*/*" {
            return true;
        };
        let Some((ty, sub)) = range.split_once('/') else {
            return false;
        };
        (ty.eq_ignore_ascii_case(want_type) && sub == "*")
            || (ty.eq_ignore_ascii_case(want_type) && sub.eq_ignore_ascii_case(want_sub))
    })
}

#[cfg(test)]
mod permission_auth_tests {
    use super::authorize_route;
    use auth::SessionSnapshot;
    use language_core::RouteAuth;

    fn session(roles: &[&str]) -> SessionSnapshot {
        session_with_mfa(roles, false)
    }

    fn session_with_mfa(roles: &[&str], mfa_verified: bool) -> SessionSnapshot {
        SessionSnapshot {
            id: "a".repeat(64),
            csrf_token: "b".repeat(64),
            principal: Some("alice".into()),
            mfa_verified,
            roles: roles.iter().map(|role| (*role).to_string()).collect(),
            memberships: vec![],
            auth_generation: 0,
        }
    }

    #[test]
    fn permission_auth_accepts_any_granting_role() {
        let policy = RouteAuth::Permission {
            name: "UserAdmin".into(),
            roles: vec!["Admin".into(), "SecurityAdmin".into()],
        };
        assert!(authorize_route(&policy, &session(&["SecurityAdmin"]), true).is_none());
    }

    #[test]
    fn permission_auth_rejects_unrelated_role() {
        let policy = RouteAuth::Permission {
            name: "UserAdmin".into(),
            roles: vec!["Admin".into()],
        };
        assert!(authorize_route(&policy, &session(&["Viewer"]), true).is_some());
    }
    #[test]
    fn permission_mfa_requires_both_granting_role_and_elevation() {
        let policy = RouteAuth::PermissionMfa {
            name: "BillingWrite".into(),
            roles: vec!["BillingAdmin".into()],
        };
        assert!(
            authorize_route(&policy, &session_with_mfa(&["BillingAdmin"], true), true).is_none()
        );
        assert!(
            authorize_route(&policy, &session_with_mfa(&["BillingAdmin"], false), true).is_some()
        );
        assert!(authorize_route(&policy, &session_with_mfa(&["Viewer"], true), true).is_some());
    }
}
