use crate::compile_source;
use language_core::Effect;

#[test]
fn infers_read_write_and_audit_effects_without_handler_boilerplate() {
    let src = r#"
model Article {
    id: i64
    title: String
}
#[query] fn byId(db: Db, id: i64) -> Result<Article, DbError> sql {
    SELECT id, title FROM articles WHERE id = :id
}
#[query] fn rename(tx: Transaction, id: i64, title: String) -> Result<Changed, DbError> mutates Article by id sql {
    UPDATE articles SET title = :title WHERE id = :id
}
#[action] fn edit(ctx: ActionContext, db: Db, id: i64, title: String) -> Result<Json, PageError> {
    let article = byId(db, id)?;
    authorize article authenticated;
    transaction db {
        rename(tx, article.id, title)?;
        audit Article article.id action rename;
    }
    return Ok(json(true));
}
route edit POST "/articles/:id<i64>" form title<String> auth user => edit;
"#;
    let program =
        compile_source(src).expect("effect inference should keep the happy path implicit");
    let action = program.action("edit").expect("action");
    assert_eq!(
        action.effects,
        vec![Effect::DbRead, Effect::DbWrite, Effect::SecurityAudit]
    );
}

#[test]
fn rejects_query_effect_without_explicit_db_capability() {
    let src = r#"
model Article {
    id: i64
    title: String
}
#[query] fn byId(db: Db, id: i64) -> Result<Article, DbError> sql {
    SELECT id, title FROM articles WHERE id = :id
}
#[page] fn show(ctx: PageContext, id: i64) -> Result<Json, PageError> {
    let article = byId(db, id)?;
    return Ok(json(expose(article, id, title)));
}
route show GET "/articles/:id<i64>" public => show;
"#;
    let error = compile_source(src).expect_err("identifier spelling must not create DB authority");
    assert!(error.to_string().contains("SEC-EFFECT-001"));
}

#[test]
fn rejects_transaction_effect_without_explicit_db_capability() {
    let src = r#"
model Article {
    id: i64
    title: String
}
#[query] fn create(tx: Transaction) -> Result<Changed, DbError> sql {
    INSERT INTO articles (id, title) VALUES (1, 'test')
}
#[action] fn edit(ctx: ActionContext) -> Result<Json, PageError> {
    transaction db {
        create(tx)?;
    }
    return Ok(json(true));
}
route edit POST "/articles" auth user => edit;
"#;
    let error = compile_source(src).expect_err("transaction authority must come from db: Db");
    assert!(
        error.to_string().contains("SEC-EFFECT-001"),
        "expected missing DB capability diagnostic, got: {error}"
    );
}
