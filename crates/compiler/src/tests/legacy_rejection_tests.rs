use super::*;

#[test]
fn legacy_set_assignment_is_rejected() {
    let src = r#"
#[page] fn home(ctx: PageContext) -> Result<Json, PageError> {
    let mut value = 0;
    set value = 1;
    return Ok(json(value));
}
route home GET "/" public => home;
"#;
    assert!(
        compile_source(src).is_err(),
        "legacy `set` syntax must be rejected"
    );
}

#[test]
fn legacy_int_type_spelling_is_rejected() {
    let src = r#"
#[page] fn home(ctx: PageContext, id: Int) -> Result<Json, PageError> {
    return Ok(json(id));
}
route home GET "/:id<Int>" public => home;
"#;
    assert!(
        compile_source(src).is_err(),
        "legacy `Int` spelling must be rejected"
    );
}

#[test]
fn legacy_query_return_spellings_are_rejected() {
    for return_type in [
        "Result<Void, DbError>",
        "Result<Product?, DbError>",
        "Result<List<Product>, DbError>",
    ] {
        let src = format!(
            r#"
model Product {{ id: i64 }}
#[query]
fn lookup(db: Db) -> {return_type} sql {{
    SELECT id FROM Product LIMIT 1
}}
"#
        );
        assert!(
            compile_source(&src).is_err(),
            "legacy query return spelling `{return_type}` must be rejected"
        );
    }
}

#[test]
fn rust_query_return_spellings_are_accepted() {
    let source = r#"
model Product { id: i64 }
#[query]
fn one(db: Db) -> Result<Option<Product>, DbError> sql {
    SELECT id FROM Product WHERE id = 1
}
#[query]
fn many(db: Db) -> Result<Vec<Product>, DbError> sql {
    SELECT id FROM Product LIMIT 8
}
#[page] fn home(ctx: PageContext) -> Result<Json, PageError> {
    return Ok(json(true));
}
route home GET "/" public => home;
"#;
    compile_source(source).expect("Rust-like Option/Vec query return spellings should compile");
}
