use super::*;

#[test]
fn simple_statements_require_semicolons() {
    let missing_let = r#"
#[page] fn home(ctx: PageContext) -> Result<Json, PageError> {
let value = 42
return Ok(json(value));
}
route home GET "/" public => home;
"#;
    assert!(compile_source(missing_let).is_err());

    let missing_return = r#"
#[page] fn home(ctx: PageContext) -> Result<Json, PageError> {
let value = 42;
return Ok(json(value))
}
route home GET "/" public => home;
"#;
    assert!(compile_source(missing_return).is_err());
}

#[test]
fn multiline_expression_uses_semicolon_not_newline_as_terminator() {
    let src = r#"
#[page] fn home(ctx: PageContext) -> Result<Json, PageError> {
let value = 10
    + 20
    * 3;
return Ok(json(value));
}
route home GET "/" public => home;
"#;
    assert!(compile_source(src).is_ok());
}

#[test]
fn block_statements_do_not_require_trailing_semicolons() {
    let src = r#"
#[page] fn home(ctx: PageContext) -> Result<Json, PageError> {
let value = 0;
if true {
    value = 1;
}
while value < 2 {
    value = value + 1;
}
return Ok(json(value));
}
route home GET "/" public => home;
"#;
    assert!(compile_source(src).is_ok());
}

#[test]
fn route_declarations_require_semicolons() {
    let missing = r#"
#[page] fn home(ctx: PageContext) -> Result<Json, PageError> {
    return Ok(json(1));
}
route home GET "/" public => home
"#;
    let err = compile_source(missing).expect_err("route without semicolon must fail");
    assert!(
        err.to_string()
            .contains("route declaration must end with `;`"),
        "{err}"
    );
}

#[test]
fn multiline_route_is_terminated_by_semicolon_not_newline() {
    let src = r#"
#[page] fn home(ctx: PageContext, q: String) -> Result<Json, PageError> {
    return Ok(json(q));
}
route home GET "/"
    query q<String>
    public
    => home;
"#;
    assert!(compile_source(src).is_ok());
}

#[test]
fn semicolon_inside_vec_macro_is_not_statement_terminator() {
    let src = r#"
#[page] fn home(ctx: PageContext) -> Result<Json, PageError> {
let mut values = vec![0.0f32; 4096];
values[0] = 1.0f32;
return Ok(json(values.len()));
}
route home GET "/" public => home;
"#;
    assert!(compile_source(src).is_ok());
}
