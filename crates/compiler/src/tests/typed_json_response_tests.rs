use super::*;

const GOOD: &str = r#"
struct EchoInput {
    name: String,
    age: i64,
    active: bool,
}
struct EchoOutput {
    name: String,
    age: i64,
    active: bool,
}
#[action]
fn echo(ctx: ActionContext, input: Json<EchoInput>) -> Result<Json<EchoOutput>, PageError> {
    let clean_name = input.name.trim();
    Ok(Json(EchoOutput { name: clean_name, age: input.age, active: input.active }))
}
route echo POST "/echo" json EchoInput auth user => echo;
"#;

#[test]
fn typed_json_response_compiles_and_reaches_verified_ir() {
    let verified = crate::compile_verified_source(GOOD).expect("typed JSON response must verify");
    let handler = verified.handler("echo").expect("verified handler");
    let body = handler
        .native_scalar_body()
        .expect("typed JSON response must lower to native scalar IR");
    assert!(matches!(
        body.statements().last(),
        Some(executable_ir::ScalarStatement::ReturnTypedJson { .. })
    ));
}

#[test]
fn typed_json_response_rejects_wrong_declared_schema() {
    let bad = GOOD.replace(
        "Result<Json<EchoOutput>, PageError>",
        "Result<Json<EchoInput>, PageError>",
    );
    assert!(crate::compile_source(&bad).is_err());
}

#[test]
fn typed_json_response_rejects_missing_field() {
    let bad = GOOD.replace(", active: input.active", "");
    assert!(crate::compile_source(&bad).is_err());
}

#[test]
fn typed_json_response_rejects_wrong_field_type() {
    let bad = GOOD.replace("age: input.age", "age: input.name");
    assert!(crate::compile_source(&bad).is_err());
}

// Typed Query<T>/Form<T> request-boundary tests.
#[test]
fn typed_query_struct_flattens_into_get_handler_and_route() {
    let src = r#"
struct SearchParams {
    q: String,
    page: i64,
}

#[page]
fn search(ctx: PageContext, input: Query<SearchParams>) -> Result<Html, PageError> {
    return Ok(html {<p>{{ input.q }} {{ input.page }}</p>});
}

route search GET "/search" query SearchParams public => search;
"#;
    let p = compile_source(src).expect("typed Query<T> should compile");
    let page = p.page("search").unwrap();
    assert_eq!(page.params.len(), 2);
    assert_eq!(page.params[0].name, "q");
    assert_eq!(page.params[0].ty, ValueType::String);
    assert_eq!(page.params[1].name, "page");
    assert_eq!(page.params[1].ty, ValueType::Int);
    let route = p.routes.iter().find(|r| r.name == "search").unwrap();
    assert_eq!(route.query_fields.len(), 2);
}

#[test]
fn typed_form_struct_flattens_into_post_handler_and_route() {
    let src = r#"
struct LoginInput {
    email: String,
    remember: bool,
}

#[action]
fn login(ctx: ActionContext, input: Form<LoginInput>) -> Result<Redirect, PageError> {
    let normalized = input.email.trim();
    return Ok(redirect(loginPage()));
}

#[page]
fn loginPage(ctx: PageContext) -> Result<Html, PageError> {
    return Ok(html {<p>Login</p>});
}

route loginPage GET "/login" public => loginPage;
route login POST "/login" form LoginInput public => login;
"#;
    let p = compile_source(src).expect("typed Form<T> should compile");
    let action = p.action("login").unwrap();
    assert_eq!(action.params.len(), 2);
    assert_eq!(action.params[0].name, "email");
    assert_eq!(action.params[1].name, "remember");
    let route = p.routes.iter().find(|r| r.name == "login").unwrap();
    assert_eq!(route.form_fields.len(), 2);
    assert!(
        route.form_schema.is_none(),
        "Rust struct form uses strict typed field binding, not legacy form semantics"
    );
}

#[test]
fn query_is_get_only_and_form_is_action_only() {
    let query_on_action = r#"
struct Input { q: String }
#[action]
fn bad(ctx: ActionContext, input: Query<Input>) -> Result<Json, PageError> {
    return Ok(json(input.q));
}
route bad POST "/bad" public => bad;
"#;
    let err = compile_source(query_on_action).unwrap_err().to_string();
    assert!(err.contains("Query<T> is only valid on #[page] handlers"));

    let form_on_page = r#"
struct Input { q: String }
#[page]
fn bad(ctx: PageContext, input: Form<Input>) -> Result<Html, PageError> {
    return Ok(html {<p>{{ input.q }}</p>});
}
route bad GET "/bad" public => bad;
"#;
    let err = compile_source(form_on_page).unwrap_err().to_string();
    assert!(err.contains("Form<T> is only valid on #[action] handlers"));
}

#[test]
fn typed_multipart_struct_flattens_into_action_and_route() {
    let src = r#"
struct UploadInput {
    title: String,
    file: Upload,
    public: bool,
}

#[action]
fn save(ctx: ActionContext, input: Multipart<UploadInput>) -> Result<Json, PageError> {
    let title = input.title.trim();
    return Ok(json(input.public));
}

route save POST "/upload" multipart UploadInput to "private" public => save;
"#;
    let p = compile_source(src).expect("typed Multipart<T> should compile");
    let action = p.action("save").unwrap();
    assert_eq!(action.params.len(), 3);
    assert_eq!(action.params[0].name, "title");
    assert_eq!(action.params[1].name, "file");
    assert_eq!(action.params[1].ty, ValueType::Upload);
    assert_eq!(action.params[2].name, "public");
    let route = p.routes.iter().find(|r| r.name == "save").unwrap();
    assert_eq!(route.multipart_fields.len(), 3);
    assert!(route.multipart_schema.is_some());
    assert_eq!(route.upload.as_ref().unwrap().name, "file");
}

#[test]
fn multipart_requires_action_and_exactly_one_upload_field() {
    let on_page = r#"
struct Input { file: Upload }
#[page]
fn bad(ctx: PageContext, input: Multipart<Input>) -> Result<Html, PageError> {
    return Ok(html {<p>bad</p>});
}
route bad GET "/bad" public => bad;
"#;
    assert!(
        compile_source(on_page)
            .unwrap_err()
            .to_string()
            .contains("Multipart<T> is only valid on #[action] handlers")
    );

    let no_file = r#"
struct Input { title: String }
#[action]
fn bad(ctx: ActionContext, input: Multipart<Input>) -> Result<Json, PageError> { return Ok(json(input.title)); }
route bad POST "/bad" multipart Input to "private" public => bad;
"#;
    assert!(
        compile_source(no_file)
            .unwrap_err()
            .to_string()
            .contains("exactly one Upload/Image field")
    );
}
