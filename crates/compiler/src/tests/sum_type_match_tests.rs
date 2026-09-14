use super::*;

const SUM_TYPE: &str = r#"
enum CommitState {
    Committed
    RolledBack
    CommitUnknown
}
"#;

#[test]
fn exhaustive_sum_match_compiles_and_is_first_class() {
    let src = format!(
        r#"
{SUM_TYPE}
#[page] fn outcome(ctx: PageContext, state: CommitState) -> Result<Json, PageError> {{
    match state {{
        Committed => {{ return Ok(json("committed")); }}
        RolledBack => {{ return Ok(json("rolledBack")); }}
        CommitUnknown => {{ fail conflict; }}
    }}
}}
route outcome GET "/:state<CommitState>" public => outcome;
"#
    );
    let program = compile_source(&src).expect("exhaustive sum match should compile");
    let (_, definition) = program.enum_by_name("CommitState").unwrap();
    assert_eq!(
        definition.variants,
        vec!["Committed", "RolledBack", "CommitUnknown"]
    );
}

#[test]
fn non_exhaustive_sum_match_is_rejected() {
    let src = format!(
        r#"
{SUM_TYPE}
#[page] fn outcome(ctx: PageContext, state: CommitState) -> Result<Json, PageError> {{
    match state {{
        Committed => {{ return Ok(json(true)); }}
        RolledBack => {{ return Ok(json(false)); }}
    }}
}}
route outcome GET "/:state<CommitState>" public => outcome;
"#
    );
    let error = compile_source(&src).unwrap_err().to_string();
    assert!(error.contains("SEC-A10-023"), "{error}");
    assert!(error.contains("CommitUnknown"), "{error}");
}

#[test]
fn duplicate_and_unknown_sum_arms_are_rejected() {
    let duplicate = format!(
        r#"
{SUM_TYPE}
#[page] fn outcome(ctx: PageContext, state: CommitState) -> Result<Json, PageError> {{
    match state {{
        Committed => {{ return Ok(json(true)); }}
        Committed => {{ return Ok(json(true)); }}
        RolledBack => {{ return Ok(json(false)); }}
        CommitUnknown => {{ fail conflict; }}
    }}
}}
route outcome GET "/:state<CommitState>" public => outcome;
"#
    );
    assert!(
        compile_source(&duplicate)
            .unwrap_err()
            .to_string()
            .contains("SEC-A10-022")
    );

    let unknown = format!(
        r#"
{SUM_TYPE}
#[page] fn outcome(ctx: PageContext, state: CommitState) -> Result<Json, PageError> {{
    match state {{
        Committed => {{ return Ok(json(true)); }}
        RolledBack => {{ return Ok(json(false)); }}
        CommitUnknown => {{ fail conflict; }}
        Maybe => {{ fail badRequest; }}
    }}
}}
route outcome GET "/:state<CommitState>" public => outcome;
"#
    );
    assert!(
        compile_source(&unknown)
            .unwrap_err()
            .to_string()
            .contains("SEC-A10-021")
    );
}

#[test]
fn match_requires_sum_type_and_all_arms_must_match_handler_return_contract() {
    let scalar = r#"
#[page] fn outcome(ctx: PageContext, state: i64) -> Result<Json, PageError> {
    match state {
        One => { return Ok(json(true)); }
    }
}
route outcome GET "/:state<i64>" public => outcome;
"#;
    assert!(
        compile_source(scalar)
            .unwrap_err()
            .to_string()
            .contains("SEC-A10-020")
    );

    let mismatch = format!(
        r#"
{SUM_TYPE}
#[page] fn outcome(ctx: PageContext, state: CommitState) -> Result<Json, PageError> {{
    match state {{
        Committed => {{ return Ok(json(true)); }}
        RolledBack => {{ return Ok(html {{rolled back}}); }}
        CommitUnknown => {{ fail conflict; }}
    }}
}}
route outcome GET "/:state<CommitState>" public => outcome;
"#
    );
    assert!(compile_source(&mismatch).is_err());
}
