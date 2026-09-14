use crate::compile_source;

#[test]
fn critical_operation_can_require_route_idempotency() {
    let src = r#"
critical Payment {
    idempotency
}

#[action] fn charge(ctx: ActionContext) -> Result<Json, PageError> critical Payment {
    return Ok(json(true));
}

route charge POST "/charge" auth critical Payment idempotent => charge;
"#;
    let program = compile_source(src).expect("critical idempotent route should compile");
    assert!(program.routes[0].idempotent);
    assert!(program.critical_operations[0].idempotency_required);
}

#[test]
fn critical_idempotency_requirement_cannot_be_omitted() {
    let src = r#"
critical Payment {
    idempotency
}

#[action] fn charge(ctx: ActionContext) -> Result<Json, PageError> critical Payment {
    return Ok(json(true));
}

route charge POST "/charge" auth critical Payment => charge;
"#;
    let err = compile_source(src).expect_err("critical idempotency requirement must be enforced");
    assert!(err.to_string().contains("SEC-A06-012"), "{err}");
}

#[test]
fn public_route_cannot_use_idempotency_keys() {
    let src = r#"
#[action] fn charge(ctx: ActionContext) -> Result<Json, PageError> {
    return Ok(json(true));
}

route charge POST "/charge" public idempotent => charge;
"#;
    let err = compile_source(src).expect_err("public idempotency keys are not principal scoped");
    assert!(err.to_string().contains("SEC-A06-011"), "{err}");
}
