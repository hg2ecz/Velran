use observability::{ActivityEvent, audit_log, json_line, utc_timestamp};

pub(super) fn audit_auth_activity(request_id: &str, actor: &str, outcome: &str, client_ip: &str) {
    audit_auth_activity_action(request_id, actor, "login", outcome, client_ip);
}

pub(super) fn audit_auth_activity_action(
    request_id: &str,
    actor: &str,
    action: &str,
    outcome: &str,
    client_ip: &str,
) {
    if let Ok(line) = json_line(&ActivityEvent {
        schema_version: 1,
        timestamp: utc_timestamp(),
        event: "user_activity",
        request_id,
        actor,
        action,
        target: "authentication",
        outcome,
        client_ip,
    }) {
        audit_log(&line);
    }
}

pub(super) fn observe_auth_security(
    category: &str,
    outcome: &str,
    request_id: &str,
    source: &str,
    principal: &str,
) {
    let correlation = format!("{source}\0{principal}");
    crate::security_alerts::observe(
        category,
        "authenticate",
        outcome,
        &correlation,
        request_id,
        "/__velran/auth/login",
    );
}
