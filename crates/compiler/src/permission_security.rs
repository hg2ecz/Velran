use crate::diagnostics::CompileError;
use language_core::{HttpMethod, Permission, Program, Route, RouteAuth};

pub(super) fn validate_handler_permission(
    route: &Route,
    program: &Program,
) -> Result<(), CompileError> {
    let required = required_permission(route, program);
    let Some(required) = required else {
        return Ok(());
    };
    let permission = program.permission(required).ok_or_else(|| {
        CompileError::security(
            "SEC-A01-020",
            format!(
                "handler `{}` requires unknown permission `{required}`",
                route.handler
            ),
            None,
        )
    })?;
    if route_auth_satisfies_permission(&route.auth, permission) {
        return Ok(());
    }
    Err(CompileError::security(
        "SEC-A01-022",
        format!(
            "route `{}` does not satisfy handler `{}` permission `{}`",
            route.name, route.handler, permission.name
        ),
        Some(format!(
            "use `auth permission {}`; roles that grant it: {}",
            short_symbol_name(&permission.name),
            permission.roles.join(", ")
        )),
    ))
}

fn required_permission<'a>(route: &Route, program: &'a Program) -> Option<&'a str> {
    match route.method {
        HttpMethod::Get => program
            .page(&route.handler)
            .and_then(|handler| handler.security.required_permission.as_deref()),
        HttpMethod::Post => program
            .action(&route.handler)
            .and_then(|handler| handler.security.required_permission.as_deref()),
    }
}

fn route_auth_satisfies_permission(auth: &RouteAuth, permission: &Permission) -> bool {
    match auth {
        RouteAuth::Permission { name, .. } | RouteAuth::PermissionMfa { name, .. } => {
            name == &permission.name
        }
        RouteAuth::Role(role) => permission.roles.iter().any(|allowed| allowed == role),
        RouteAuth::Public | RouteAuth::Webhook(_) | RouteAuth::User | RouteAuth::Mfa => false,
    }
}

fn short_symbol_name(symbol: &str) -> &str {
    symbol.rsplit("::").next().unwrap_or(symbol)
}
