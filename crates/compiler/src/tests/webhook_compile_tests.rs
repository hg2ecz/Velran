use crate::compile_source;
use language_core::RouteAuth;

#[test]
fn verified_webhook_route_compiles_with_explicit_boundary() {
    let src = r#"
webhook BillingEvents {
    verified by BillingWebhookKey
    signatureHeader "x-billing-signature"
    timestampHeader "x-billing-timestamp"
    replayWindow 300
}

#[action] fn billing(ctx: ActionContext, event: String) -> Result<Json, PageError> {
    return Ok(json(event));
}

route billing POST "/webhooks/billing" json event<String> webhook BillingEvents => billing;
"#;
    let program = compile_source(src).expect("verified webhook route should compile");
    assert_eq!(program.webhooks.len(), 1);
    assert_eq!(program.webhooks[0].replay_window_secs, 300);
    assert!(matches!(program.routes[0].auth, RouteAuth::Webhook(_)));
}

#[test]
fn webhook_route_rejects_unknown_policy() {
    let src = r#"
#[action] fn billing(ctx: ActionContext, event: String) -> Result<Json, PageError> {
    return Ok(json(event));
}
route billing POST "/webhooks/billing" json event<String> webhook Missing => billing;
"#;
    let err = compile_source(src).expect_err("unknown webhook must fail closed");
    assert!(err.to_string().contains("SEC-A08-022"));
}

#[test]
fn webhook_route_requires_typed_json_body() {
    let src = r#"
webhook BillingEvents {
    verified by BillingWebhookKey
}
#[action] fn billing(ctx: ActionContext) -> Result<Json, PageError> {
    return Ok(json("ok"));
}
route billing POST "/webhooks/billing" webhook BillingEvents => billing;
"#;
    let err = compile_source(src).expect_err("untyped webhook body must fail closed");
    assert!(err.to_string().contains("SEC-A08-025"));
}

#[test]
fn webhook_replay_window_is_bounded() {
    let src = r#"
webhook BillingEvents {
    verified by BillingWebhookKey
    replayWindow 86400
}
#[action] fn billing(ctx: ActionContext, event: String) -> Result<Json, PageError> {
    return Ok(json(event));
}
route billing POST "/webhooks/billing" json event<String> webhook BillingEvents => billing;
"#;
    let err = compile_source(src).expect_err("wide replay window must fail closed");
    assert!(err.to_string().contains("SEC-A08-021"));
}

#[test]
fn webhook_cannot_stack_client_idempotency() {
    let src = r#"
webhook BillingEvents {
    verified by BillingWebhookKey
}
#[action] fn billing(ctx: ActionContext, event: String) -> Result<Json, PageError> {
    return Ok(json(event));
}
route billing POST "/webhooks/billing" json event<String> webhook BillingEvents idempotent => billing;
"#;
    let err = compile_source(src).expect_err("replay authorities must not be stacked");
    assert!(err.to_string().contains("SEC-A08-024"));
}
