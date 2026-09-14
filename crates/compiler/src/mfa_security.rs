use crate::diagnostics::CompileError;
use language_core::{HttpMethod, Program, Route, RouteAuth};

pub(super) fn validate_handler_mfa(route: &Route, program: &Program) -> Result<(), CompileError> {
    if !handler_requires_mfa(route, program) {
        return Ok(());
    }
    if route_auth_satisfies_mfa(&route.auth) {
        return Ok(());
    }
    Err(CompileError::security(
        "SEC-A07-020",
        format!(
            "route `{}` does not provide MFA elevation required by handler `{}`",
            route.name, route.handler
        ),
        Some(mfa_help(route, program)),
    ))
}

fn handler_requires_mfa(route: &Route, program: &Program) -> bool {
    match route.method {
        HttpMethod::Get => program
            .page(&route.handler)
            .is_some_and(|h| h.security.mfa_required),
        HttpMethod::Post => program
            .action(&route.handler)
            .is_some_and(|h| h.security.mfa_required),
    }
}

fn route_auth_satisfies_mfa(auth: &RouteAuth) -> bool {
    matches!(auth, RouteAuth::Mfa | RouteAuth::PermissionMfa { .. })
}

fn mfa_help(route: &Route, program: &Program) -> String {
    let required_permission = match route.method {
        HttpMethod::Get => program
            .page(&route.handler)
            .and_then(|h| h.security.required_permission.as_deref()),
        HttpMethod::Post => program
            .action(&route.handler)
            .and_then(|h| h.security.required_permission.as_deref()),
    };
    match required_permission {
        Some(permission) => format!(
            "use `auth permission {} mfa`; the platform verifies the existing session's MFA elevation",
            short_symbol_name(permission)
        ),
        None => "use `auth mfa`; the platform verifies the existing session's MFA elevation".into(),
    }
}

fn short_symbol_name(symbol: &str) -> &str {
    symbol.rsplit("::").next().unwrap_or(symbol)
}
