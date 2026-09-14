use crate::compile_source;

#[test]
fn list_query_requires_literal_row_bound() {
    let source = r#"
model Item { id: i64 }
#[query] fn items(db: Db) -> Result<Vec<Item>, DbError> sql {
    SELECT id FROM items
}
#[page] fn index(ctx: PageContext, db: Db) -> Result<Json, PageError> {
    let items = items(db)?;
    return Ok(json(expose(items, id)));
}
route index GET "/" public => index;
"#;
    let error = compile_source(source).expect_err("unbounded List<Model> query must be rejected");
    assert!(error.to_string().contains("SEC-RESOURCE-001"));
}

#[test]
fn list_query_rejects_parameterized_limit() {
    let source = r#"
model Item { id: i64 }
#[query] fn items(db: Db, limit: i64) -> Result<Vec<Item>, DbError> sql {
    SELECT id FROM items LIMIT :limit
}
#[page] fn index(ctx: PageContext, db: Db, limit: i64) -> Result<Json, PageError> {
    let items = items(db, limit)?;
    return Ok(json(expose(items, id)));
}
route index GET "/" query limit<i64> validate limit range 1 1000 public => index;
"#;
    let error = compile_source(source).expect_err("parameterized row limit must be rejected");
    assert!(error.to_string().contains("SEC-RESOURCE-001"));
}

#[test]
fn list_query_rejects_oversized_literal_limit() {
    let source = r#"
model Item { id: i64 }
#[query] fn items(db: Db) -> Result<Vec<Item>, DbError> sql {
    SELECT id FROM items LIMIT 1001
}
#[page] fn index(ctx: PageContext, db: Db) -> Result<Json, PageError> {
    let items = items(db)?;
    return Ok(json(expose(items, id)));
}
route index GET "/" public => index;
"#;
    let error = compile_source(source).expect_err("oversized literal row limit must be rejected");
    assert!(error.to_string().contains("SEC-RESOURCE-001"));
}

#[test]
fn list_query_accepts_literal_limit_at_or_below_hard_cap() {
    let source = r#"
model Item { id: i64 }
#[query] fn items(db: Db) -> Result<Vec<Item>, DbError> sql {
    SELECT id FROM items LIMIT 1000
}
#[page] fn index(ctx: PageContext, db: Db) -> Result<Json, PageError> {
    let items = items(db)?;
    return Ok(json(expose(items, id)));
}
route index GET "/" public => index;
"#;
    compile_source(source).expect("bounded List<Model> query should compile");
}
