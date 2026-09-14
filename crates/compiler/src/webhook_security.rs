use crate::diagnostics::CompileError;
use language_core::{HttpMethod, Program, Route, RouteAuth};

pub(super) fn validate_route(route: &Route, program: &Program) -> Result<(), CompileError> {
    let RouteAuth::Webhook(name) = &route.auth else {
        return Ok(());
    };
    if route.method != HttpMethod::Post {
        return Err(CompileError::security(
            "SEC-A08-023",
            format!("webhook route `{}` must use POST", route.name),
            Some("verified webhook boundaries are state-changing POST endpoints".into()),
        ));
    }
    if route.json_fields.is_empty() {
        return Err(CompileError::security(
            "SEC-A08-025",
            format!("webhook route `{}` must declare a typed JSON body", route.name),
            Some(
                "declare `json field<Type> ...`; signature verification runs on the raw bytes before typed decoding"
                    .into(),
            ),
        ));
    }
    if route.tenant_field.is_some() {
        return Err(CompileError::security(
            "SEC-A01-040",
            format!(
                "webhook route `{}` cannot derive tenant authority from the request path",
                route.name
            ),
            Some(
                "resolve tenant authority from verified webhook payload/provider mapping inside the handler, not from an unauthenticated URL claim"
                    .into(),
            ),
        ));
    }
    if route.idempotent {
        return Err(CompileError::security(
            "SEC-A08-024",
            format!(
                "webhook route `{}` must not also use client idempotency keys",
                route.name
            ),
            Some(
                "webhook replay protection is platform-owned and keyed by the verified signature/timestamp"
                    .into(),
            ),
        ));
    }
    if program.webhook(name).is_none() {
        return Err(CompileError::security(
            "SEC-A08-022",
            format!("route `{}` references unknown webhook `{name}`", route.name),
            None,
        ));
    }
    Ok(())
}
