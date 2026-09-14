use crate::diagnostics::CompileError;
use language_core::{HttpMethod, Program, Route, RouteAuth};

pub(super) fn validate_route(route: &Route, program: &Program) -> Result<(), CompileError> {
    if route.idempotent {
        if route.method != HttpMethod::Post {
            return Err(CompileError::security(
                "SEC-A06-010",
                format!("route `{}` uses idempotency but is not POST", route.name),
                Some("idempotency guards are for state-changing POST operations".into()),
            ));
        }
        if matches!(route.auth, RouteAuth::Public) {
            return Err(CompileError::security(
                "SEC-A06-011",
                format!(
                    "route `{}` cannot be both public and idempotent",
                    route.name
                ),
                Some(
                    "authenticate the caller so idempotency keys are scoped to a stable principal"
                        .into(),
                ),
            ));
        }
    }

    let Some(action) = program.action(&route.handler) else {
        return Ok(());
    };
    let Some(critical_name) = action.security.critical_operation.as_deref() else {
        return Ok(());
    };
    let Some(operation) = program.critical_operation(critical_name) else {
        return Ok(());
    };
    if operation.idempotency_required && !route.idempotent {
        return Err(CompileError::security(
            "SEC-A06-012",
            format!(
                "critical route `{}` must use idempotency for `{}`",
                route.name, operation.name
            ),
            Some("add `idempotent` to the route; the platform will enforce the Idempotency-Key replay guard".into()),
        ));
    }
    Ok(())
}
