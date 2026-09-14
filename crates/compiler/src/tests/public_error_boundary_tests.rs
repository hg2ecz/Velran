use crate::compile_source;

#[test]
fn page_can_fail_with_public_not_found() {
    let src = r#"
#[page] fn show(ctx: PageContext) -> Result<Json, PageError> {
    fail notFound;
}
route show GET "/show" public => show;
"#;
    compile_source(src).expect("public fail should compile");
}

#[test]
fn action_can_fail_with_public_conflict() {
    let src = r#"
#[action] fn save(ctx: ActionContext) -> Result<Json, PageError> {
    fail conflict;
}
route save POST "/save" public => save;
"#;
    compile_source(src).expect("public fail should compile");
}

#[test]
fn internal_error_name_is_not_exposable() {
    let src = r#"
#[page] fn show(ctx: PageContext) -> Result<Json, PageError> {
    fail internal;
}
route show GET "/show" public => show;
"#;
    let error =
        compile_source(src).expect_err("internal errors must not be public response values");
    assert!(error.to_string().contains("SEC-A10-003"));
}

#[test]
fn arbitrary_public_error_name_is_rejected() {
    let src = r#"
#[page] fn show(ctx: PageContext) -> Result<Json, PageError> {
    fail databaseUnavailable;
}
route show GET "/show" public => show;
"#;
    let error = compile_source(src).expect_err("public errors are a closed language set");
    assert!(error.to_string().contains("SEC-A10-003"));
}
