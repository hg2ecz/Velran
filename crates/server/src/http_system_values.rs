use auth::SessionSnapshot;
use language_core::Value;

pub(super) fn base_system_values(
    session: &SessionSnapshot,
    request_id: &str,
) -> Vec<(String, Value)> {
    vec![
        (
            "csrfToken".to_string(),
            Value::String(session.csrf_token.clone()),
        ),
        (
            "authPrincipal".to_string(),
            Value::String(session.principal.clone().unwrap_or_default()),
        ),
        (
            "authMfaVerified".to_string(),
            Value::Bool(session.mfa_verified),
        ),
        (
            "__authRoles".to_string(),
            Value::List(session.roles.iter().cloned().map(Value::String).collect()),
        ),
        (
            "__authMemberships".to_string(),
            Value::List(
                session
                    .memberships
                    .iter()
                    .map(|tenant| Value::String(tenant.as_str().to_string()))
                    .collect(),
            ),
        ),
        (
            "__requestId".to_string(),
            Value::String(request_id.to_string()),
        ),
    ]
}

pub(super) fn append_flash(values: &mut Vec<(String, Value)>, flash: Option<auth::SessionFlash>) {
    if let Some(flash) = flash {
        values.push(("__flashKind".to_string(), Value::String(flash.kind)));
        values.push(("__flashMessage".to_string(), Value::String(flash.message)));
    }
}
