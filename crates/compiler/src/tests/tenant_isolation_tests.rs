use super::*;

const TYPES_AND_MODEL: &str = r#"
type OrganizationId = String {
    length 1 64;
    pattern "^[a-z0-9_-]+$";
}

model Article scoped by organizationId {
    id: i64
    organizationId: OrganizationId
    title: String
}
"#;

#[test]
fn tenant_bound_route_can_call_scoped_query() {
    let src = format!(
        r#"{}
#[query] fn loadArticle(db: Db, organizationId: OrganizationId, id: i64) -> Result<Article, DbError> sql {{
    SELECT id, organizationId, title
    FROM articles
    WHERE organizationId = :organizationId AND id = :id
}}

#[page] fn show(ctx: PageContext, db: Db, organizationId: OrganizationId, id: i64) -> Result<Json, PageError> {{
    let article = loadArticle(db, organizationId, id)?;
    return Ok(json(article.id));
}}

route show GET "/org/:organizationId<OrganizationId>/articles/:id<i64>"
    tenant organizationId
    auth user
    => show;
"#,
        TYPES_AND_MODEL
    );
    compile_source(&src).expect("active tenant proof should satisfy scoped query");
}

#[test]
fn scoped_query_rejects_non_tenant_route_input() {
    let src = format!(
        r#"{}
#[query] fn loadArticle(db: Db, organizationId: OrganizationId, id: i64) -> Result<Article, DbError> sql {{
    SELECT id, organizationId, title
    FROM articles
    WHERE organizationId = :organizationId AND id = :id
}}

#[page] fn show(ctx: PageContext, db: Db, organizationId: OrganizationId, id: i64) -> Result<Json, PageError> {{
    let article = loadArticle(db, organizationId, id)?;
    return Ok(json(article.id));
}}

route show GET "/org/:organizationId<OrganizationId>/articles/:id<i64>"
    auth user
    => show;
"#,
        TYPES_AND_MODEL
    );
    let err =
        compile_source(&src).expect_err("validated input alone must not become tenant authority");
    assert!(err.to_string().contains("SEC-A01-039"), "{err}");
}

#[test]
fn scoped_query_requires_database_tenant_guard() {
    let src = format!(
        r#"{}
#[query] fn loadArticle(db: Db, organizationId: OrganizationId, id: i64) -> Result<Article, DbError> sql {{
    SELECT id, organizationId, title
    FROM articles
    WHERE id = :id AND :organizationId = :organizationId
}}

#[page] fn show(ctx: PageContext, db: Db, organizationId: OrganizationId, id: i64) -> Result<Json, PageError> {{
    let article = loadArticle(db, organizationId, id)?;
    return Ok(json(article.id));
}}

route show GET "/org/:organizationId<OrganizationId>/articles/:id<i64>"
    tenant organizationId
    auth user
    => show;
"#,
        TYPES_AND_MODEL
    );
    let err = compile_source(&src).expect_err("bind presence is not a tenant guard");
    assert!(err.to_string().contains("SEC-A01-038"), "{err}");
}

#[test]
fn tenant_bound_route_cannot_be_public() {
    let src = r#"
type OrganizationId = String { length 1 64; }
#[page] fn show(ctx: PageContext, organizationId: OrganizationId) -> Result<Json, PageError> {
    return Ok(json(true));
}
route show GET "/org/:organizationId<OrganizationId>"
    tenant organizationId
    public
    => show;
"#;
    let err = compile_source(src).expect_err("tenant membership requires authenticated principal");
    assert!(err.to_string().contains("SEC-A01-032"), "{err}");
}
