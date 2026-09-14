use crate::compile_source;

const QUERY: &str = r#"
#[query] fn record(tx: Transaction) -> Result<Changed, DbError> sql {
    INSERT INTO audit_marker (id) VALUES (1)
}
"#;

#[test]
fn captured_transaction_outcome_is_exhaustively_matchable() {
    let src = format!(
        r#"
{QUERY}
critical Payment {{
    transaction
    idempotency
}}

#[action] fn charge(ctx: ActionContext, db: Db) -> Result<Json, PageError> critical Payment {{
    let outcome = transaction db {{
        record(tx)?;
    }};
    match outcome {{
        Committed => {{ return Ok(json(true)); }}
        RolledBack => {{ fail conflict; }}
        CommitUnknown => {{ fail conflict; }}
    }}
}}
route charge POST "/charge" auth critical Payment idempotent => charge;
"#
    );
    compile_source(&src).expect("critical idempotent transaction outcome should compile");
}

#[test]
fn idempotent_critical_transaction_requires_outcome_capture() {
    let src = format!(
        r#"
{QUERY}
critical Payment {{
    transaction
    idempotency
}}
#[action] fn charge(ctx: ActionContext, db: Db) -> Result<Json, PageError> critical Payment {{
    transaction db {{ record(tx)?; }}
    return Ok(json(true));
}}
route charge POST "/charge" auth critical Payment idempotent => charge;
"#
    );
    let error = compile_source(&src).expect_err("outcome capture must be mandatory");
    assert!(error.to_string().contains("SEC-A10-024"), "{error}");
}

#[test]
fn idempotent_critical_transaction_requires_outcome_match() {
    let src = format!(
        r#"
{QUERY}
critical Payment {{
    transaction
    idempotency
}}
#[action] fn charge(ctx: ActionContext, db: Db) -> Result<Json, PageError> critical Payment {{
    let outcome = transaction db {{ record(tx)?; }};
    return Ok(json(true));
}}
route charge POST "/charge" auth critical Payment idempotent => charge;
"#
    );
    let error = compile_source(&src).expect_err("outcome match must be mandatory");
    assert!(error.to_string().contains("SEC-A10-025"), "{error}");
}
