use super::*;

#[test]
fn route_requires_explicit_access_policy() {
    let src = r#"
#[page] fn home(ctx: PageContext) -> Result<Json, PageError> {
    return Ok(json(true));
}
route home GET "/" => home;
"#;
    let err = compile_source(src).expect_err("implicit public routes must be rejected");
    let msg = err.to_string();
    assert!(msg.contains("SEC-A01-001"), "{msg}");
    assert!(msg.contains("explicit access policy"), "{msg}");
}

#[test]
fn explicit_public_route_compiles() {
    let src = r#"
#[page] fn home(ctx: PageContext) -> Result<Json, PageError> {
    return Ok(json(true));
}
route home GET "/" public => home;
"#;
    let program = compile_source(src).expect("explicit public route should compile");
    assert!(matches!(program.routes[0].auth, RouteAuth::Public));
}

#[test]
fn raw_external_string_gets_implicit_safe_bound() {
    let src = r#"
#[page] fn search(ctx: PageContext, q: String) -> Result<Json, PageError> {
    return Ok(json(q));
}
route search GET "/search" query q<String> public => search;
"#;
    let program = compile_source(src).expect("bounded request String should compile");
    let route = &program.routes[0];
    assert!(route.validations.iter().any(|rule| {
        rule.field == "q" && matches!(&rule.kind, ValidationKind::Length { min: 0, max: 4096 })
    }));
}

#[test]
fn explicit_string_bound_overrides_implicit_default() {
    let src = r#"
#[page] fn search(ctx: PageContext, q: String) -> Result<Json, PageError> {
    return Ok(json(q));
}
route search GET "/search" query q<String> validate q length 1 120 public => search;
"#;
    let program = compile_source(src).expect("explicit bounded String should compile");
    let route = &program.routes[0];
    let q_rules = route
        .validations
        .iter()
        .filter(|rule| rule.field == "q" && matches!(&rule.kind, ValidationKind::Length { .. }))
        .count();
    assert_eq!(q_rules, 1, "explicit bound must not be duplicated");
}

#[test]
fn redirect_rejects_dynamic_string_target() {
    let src = r#"
#[action] fn go(ctx: ActionContext, target: String) -> Result<Redirect, PageError> {
    return Ok(redirect(target));
}
route go POST "/go" form target<String> public => go;
"#;
    let err = compile_source(src).expect_err("dynamic redirects must fail closed");
    let msg = err.to_string();
    assert!(msg.contains("SEC-A01-011"), "{msg}");
}

#[test]
fn redirect_rejects_protocol_relative_literal() {
    let src = r#"
#[action] fn go(ctx: ActionContext) -> Result<Redirect, PageError> {
    return Ok(redirect("//evil.example"));
}
route go POST "/go" public => go;
"#;
    let err = compile_source(src).expect_err("protocol-relative redirect must fail closed");
    let msg = err.to_string();
    assert!(msg.contains("SEC-A01-011"), "{msg}");
}

#[test]
fn redirect_accepts_typed_get_route() {
    let src = r#"
#[action] fn go(ctx: ActionContext) -> Result<Redirect, PageError> {
    return Ok(redirect(account()));
}
#[page] fn account(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {account}); }
route account GET "/account" public => account;
route go POST "/go" public => go;
"#;
    compile_source(src).expect("typed redirect should compile");
}

#[test]
fn redirect_rejects_post_route_target() {
    let src = r#"
#[action] fn save(ctx: ActionContext) -> Result<Redirect, PageError> {
    return Ok(redirect(save()));
}
route save POST "/save" public => save;
"#;
    let err = compile_source(src).expect_err("redirect to POST route must fail closed");
    let msg = err.to_string();
    assert!(msg.contains("SEC-A01-008"), "{msg}");
}

#[test]
fn redirect_typechecks_route_arguments() {
    let src = r#"
#[action] fn go(ctx: ActionContext) -> Result<Redirect, PageError> {
    return Ok(redirect(account("not-an-int")));
}
#[page] fn account(ctx: PageContext, id: i64) -> Result<Html, PageError> { return Ok(html {account}); }
route account GET "/account/:id<i64>" public => account;
route go POST "/go" public => go;
"#;
    let err = compile_source(src).expect_err("typed redirect arguments must match route types");
    let msg = err.to_string();
    assert!(msg.contains("SEC-A01-010"), "{msg}");
}

#[test]
fn secret_model_field_cannot_be_returned_as_json_even_through_local_alias() {
    let src = r#"
model Account {
    id: i64
    passwordHash: Secret<String>
}
#[query] fn loadAccount(db: Db, id: i64) -> Result<Account, DbError> sql {
    SELECT id, passwordHash FROM accounts WHERE id = :id
}
#[page] fn account(ctx: PageContext, db: Db, id: i64) -> Result<Json, PageError> {
    let account = loadAccount(db, id)?;
    let leaked = account.passwordHash;
    return Ok(json(leaked));
}
route account GET "/accounts/:id<i64>" auth user => account;
"#;
    let err = compile_source(src).expect_err("Secret<T> must not cross a JSON response boundary");
    let msg = err.to_string();
    assert!(msg.contains("SEC-DATA-001"), "{msg}");
}

#[test]
fn secret_model_field_cannot_be_rendered_as_html() {
    let src = r#"
model Account {
    id: i64
    apiKey: Secret<String>
}
#[query] fn loadAccount(db: Db, id: i64) -> Result<Account, DbError> sql {
    SELECT id, apiKey FROM accounts WHERE id = :id
}
#[page] fn account(ctx: PageContext, db: Db, id: i64) -> Result<Html, PageError> {
    let account = loadAccount(db, id)?;
    return Ok(html { <p>{{ account.apiKey }}</p> });
}
route account GET "/accounts/:id<i64>" auth user => account;
"#;
    let err = compile_source(src).expect_err("Secret<T> must not cross an HTML response boundary");
    let msg = err.to_string();
    assert!(msg.contains("SEC-DATA-002"), "{msg}");
}

#[test]
fn sensitive_model_field_requires_object_authorization_before_json_disclosure() {
    let src = r#"
model Profile {
    id: i64
    owner: String
    email: Sensitive<Email>
}
#[query] fn loadProfile(db: Db, id: i64) -> Result<Profile, DbError> sql {
    SELECT id, owner, email FROM profiles WHERE id = :id
}
#[page] fn profile(ctx: PageContext, db: Db, id: i64) -> Result<Json, PageError> {
    let profile = loadProfile(db, id)?;
    return Ok(json(profile.email));
}
route profile GET "/profiles/:id<i64>" auth user => profile;
"#;
    let err = compile_source(src).expect_err("Sensitive<T> disclosure needs authorization proof");
    let msg = err.to_string();
    assert!(msg.contains("SEC-A01-004"), "{msg}");
}

#[test]
fn authorize_refines_model_for_sensitive_json_disclosure() {
    let src = r#"
model Profile {
    id: i64
    owner: String
    email: Sensitive<Email>
}
#[query] fn loadProfile(db: Db, id: i64) -> Result<Profile, DbError> sql {
    SELECT id, owner, email FROM profiles WHERE id = :id
}
#[page] fn profile(ctx: PageContext, db: Db, id: i64) -> Result<Json, PageError> {
    let profile = loadProfile(db, id)?;
    authorize profile owner owner;
    return Ok(json(profile.email));
}
route profile GET "/profiles/:id<i64>" auth user => profile;
"#;
    compile_source(src).expect("authorized Sensitive<T> field should cross JSON boundary");
}

#[test]
fn authorization_proof_survives_sensitive_local_alias() {
    let src = r#"
model Profile {
    id: i64
    owner: String
    email: Sensitive<Email>
}
#[query] fn loadProfile(db: Db, id: i64) -> Result<Profile, DbError> sql {
    SELECT id, owner, email FROM profiles WHERE id = :id
}
#[page] fn profile(ctx: PageContext, db: Db, id: i64) -> Result<Html, PageError> {
    let profile = loadProfile(db, id)?;
    authorize profile owner owner;
    let email = profile.email;
    return Ok(html { <p>{{ email }}</p> });
}
route profile GET "/profiles/:id<i64>" auth user => profile;
"#;
    compile_source(src).expect("authorization proof should propagate through local aliases");
}

#[test]
fn sensitive_html_disclosure_without_authorization_is_rejected() {
    let src = r#"
model Profile {
    id: i64
    owner: String
    email: Sensitive<Email>
}
#[query] fn loadProfile(db: Db, id: i64) -> Result<Profile, DbError> sql {
    SELECT id, owner, email FROM profiles WHERE id = :id
}
#[page] fn profile(ctx: PageContext, db: Db, id: i64) -> Result<Html, PageError> {
    let profile = loadProfile(db, id)?;
    return Ok(html { <p>{{ profile.email }}</p> });
}
route profile GET "/profiles/:id<i64>" auth user => profile;
"#;
    let err =
        compile_source(src).expect_err("Sensitive<T> HTML disclosure needs authorization proof");
    let msg = err.to_string();
    assert!(msg.contains("SEC-A01-004"), "{msg}");
}

#[test]
fn update_query_requires_explicit_mutation_contract() {
    let src = r#"
model Article {
    id: i64
    title: String
}
#[query] fn updateTitle(tx: Transaction, id: i64, title: String) -> Result<(), DbError> sql {
    UPDATE articles SET title = :title WHERE id = :id
}
"#;
    let err = compile_source(src).expect_err("UPDATE must declare an authorization target");
    let msg = err.to_string();
    assert!(msg.contains("SEC-A01-006"), "{msg}");
}

#[test]
fn mutation_rejects_raw_request_identifier() {
    let src = r#"
model Article {
    id: i64
    title: String
}
#[query] fn updateTitle(tx: Transaction, id: i64, title: String) -> Result<(), DbError> mutates Article by id sql {
    UPDATE articles SET title = :title WHERE id = :id
}
#[action] fn edit(ctx: ActionContext, db: Db, id: i64, title: String) -> Result<Json, PageError> {
    transaction db {
        updateTitle(tx, id, title)?;
    }
    return Ok(json(true));
}
route edit POST "/articles/:id<i64>" form title<String> auth user => edit;
"#;
    let err = compile_source(src).expect_err("request id must not authorize its own mutation");
    let msg = err.to_string();
    assert!(msg.contains("SEC-A01-005"), "{msg}");
}

#[test]
fn mutation_accepts_authorized_model_key() {
    let src = r#"
model Article {
    id: i64
    title: String
}
#[query] fn loadArticle(db: Db, id: i64) -> Result<Article, DbError> sql {
    SELECT id, title FROM articles WHERE id = :id
}
#[query] fn updateTitle(tx: Transaction, id: i64, title: String) -> Result<(), DbError> mutates Article by id sql {
    UPDATE articles SET title = :title WHERE id = :id
}
#[action] fn edit(ctx: ActionContext, db: Db, id: i64, title: String) -> Result<Json, PageError> {
    let article = loadArticle(db, id)?;
    authorize article authenticated;
    transaction db {
        updateTitle(tx, article.id, title)?;
    }
    return Ok(json(true));
}
route edit POST "/articles/:id<i64>" form title<String> auth user => edit;
"#;
    compile_source(src).expect("authorized object key should prove mutation access");
}

#[test]
fn mutation_proof_survives_unchanged_local_alias() {
    let src = r#"
model Article {
    id: i64
    title: String
}
#[query] fn loadArticle(db: Db, id: i64) -> Result<Article, DbError> sql {
    SELECT id, title FROM articles WHERE id = :id
}
#[query] fn updateTitle(tx: Transaction, id: i64, title: String) -> Result<(), DbError> mutates Article by id sql {
    UPDATE articles SET title = :title WHERE id = :id
}
#[action] fn edit(ctx: ActionContext, db: Db, id: i64, title: String) -> Result<Json, PageError> {
    let article = loadArticle(db, id)?;
    authorize article authenticated;
    let articleId = article.id;
    transaction db {
        updateTitle(tx, articleId, title)?;
    }
    return Ok(json(true));
}
route edit POST "/articles/:id<i64>" form title<String> auth user => edit;
"#;
    compile_source(src).expect("unchanged alias should preserve mutation evidence");
}

#[test]
fn mutation_rejects_transformed_authorized_key() {
    let src = r#"
model Article {
    id: i64
    title: String
}
#[query] fn loadArticle(db: Db, id: i64) -> Result<Article, DbError> sql {
    SELECT id, title FROM articles WHERE id = :id
}
#[query] fn updateTitle(tx: Transaction, id: i64, title: String) -> Result<(), DbError> mutates Article by id sql {
    UPDATE articles SET title = :title WHERE id = :id
}
#[action] fn edit(ctx: ActionContext, db: Db, id: i64, title: String) -> Result<Json, PageError> {
    let article = loadArticle(db, id)?;
    authorize article authenticated;
    let otherId = article.id + 1;
    transaction db {
        updateTitle(tx, otherId, title)?;
    }
    return Ok(json(true));
}
route edit POST "/articles/:id<i64>" form title<String> auth user => edit;
"#;
    let err = compile_source(src).expect_err("transformation must destroy mutation evidence");
    assert!(err.to_string().contains("SEC-A01-005"));
}

#[test]
fn mutation_rejects_authorization_proof_from_other_model() {
    let src = r#"
model Article {
    id: i64
    title: String
}
model Comment {
    id: i64
    body: String
}
#[query] fn loadComment(db: Db, id: i64) -> Result<Comment, DbError> sql {
    SELECT id, body FROM comments WHERE id = :id
}
#[query] fn updateTitle(tx: Transaction, id: i64, title: String) -> Result<(), DbError> mutates Article by id sql {
    UPDATE articles SET title = :title WHERE id = :id
}
#[action] fn edit(ctx: ActionContext, db: Db, id: i64, title: String) -> Result<Json, PageError> {
    let comment = loadComment(db, id)?;
    authorize comment authenticated;
    transaction db {
        updateTitle(tx, comment.id, title)?;
    }
    return Ok(json(true));
}
route edit POST "/articles/:id<i64>" form title<String> auth user => edit;
"#;
    let err =
        compile_source(src).expect_err("proof from another model must not authorize mutation");
    assert!(err.to_string().contains("SEC-A01-005"));
}

#[test]
fn path_string_gets_implicit_safe_bound() {
    let src = r#"
#[page] fn profile(ctx: PageContext, slug: String) -> Result<Json, PageError> {
    return Ok(json(slug));
}
route profile GET "/profiles/:slug<String>" public => profile;
"#;
    let program = compile_source(src).expect("path String should receive a safe default bound");
    assert!(program.routes[0].validations.iter().any(|rule| {
        rule.field == "slug" && matches!(&rule.kind, ValidationKind::Length { min: 0, max: 4096 })
    }));
}

#[test]
fn validated_route_string_can_reach_mutation_boundary() {
    let src = r#"
model Note {
    id: i64
    title: String
}
#[query] fn createNote(tx: Transaction, title: String) -> Result<Note, DbError> sql {
    INSERT INTO notes (title) VALUES (:title) RETURNING id, title
}
#[action] fn create(ctx: ActionContext, db: Db, title: String) -> Result<Json, PageError> {
    transaction db {
        let note = createNote(tx, title)?;
    }
    return Ok(json(true));
}
route create POST "/notes" form title<String> public => create;
"#;
    compile_source(src).expect("route-bound String has implicit validation proof");
}

#[test]
fn unvalidated_upload_metadata_cannot_reach_mutation_boundary() {
    let src = r#"
model Asset {
    id: i64
    filename: String
}
#[query] fn createAsset(tx: Transaction, filename: String) -> Result<Asset, DbError> sql {
    INSERT INTO assets (filename) VALUES (:filename) RETURNING id, filename
}
#[action] fn upload(ctx: ActionContext, db: Db, file: Upload) -> Result<Json, PageError> {
    transaction db {
        let asset = createAsset(tx, file.filename)?;
    }
    return Ok(json(true));
}
route upload POST "/assets" upload file<Upload> to uploads public => upload;
"#;
    let err = compile_source(src).expect_err("unvalidated upload metadata must not reach writes");
    let msg = err.to_string();
    assert!(msg.contains("SEC-DATA-003"), "{msg}");
}

#[test]
fn domain_string_type_applies_constraints_at_route_boundary() {
    let src = r#"
type Username = String {
    length 3 32
    pattern "^[A-Za-z0-9_]+$"
}
#[page] fn profile(ctx: PageContext, username: Username) -> Result<Json, PageError> {
    return Ok(json(username));
}
route profile GET "/profiles/:username<Username>" public => profile;
"#;
    let program = compile_source(src).expect("domain input type should compile");
    let route = &program.routes[0];
    assert!(route.validations.iter().any(|rule| {
        rule.field == "username" && matches!(&rule.kind, ValidationKind::Length { min: 3, max: 32 })
    }));
    assert!(route.validations.iter().any(|rule| {
        rule.field == "username"
            && matches!(&rule.kind, ValidationKind::Pattern { regex } if regex == "^[A-Za-z0-9_]+$")
    }));
}

#[test]
fn domain_int_type_applies_range_at_route_boundary() {
    let src = r#"
type PageSize = i64 {
    range 1 100
}
#[page] fn list(ctx: PageContext, size: PageSize) -> Result<Json, PageError> {
    return Ok(json(size));
}
route list GET "/items" query size<PageSize> public => list;
"#;
    let program = compile_source(src).expect("bounded domain integer should compile");
    assert!(program.routes[0].validations.iter().any(|rule| {
        rule.field == "size" && matches!(&rule.kind, ValidationKind::Range { min: 1, max: 100 })
    }));
}

#[test]
fn domain_type_rejects_constraint_that_does_not_match_base() {
    let src = r#"
type PageSize = i64 {
    length 1 3
}
#[page] fn home(ctx: PageContext) -> Result<Json, PageError> { return Ok(json(true)); }
route home GET "/" public => home;
"#;
    let err = compile_source(src).expect_err("invalid domain constraints must fail closed");
    assert!(err.to_string().contains("does not match its base type"));
}

#[test]
fn split_bounded_requires_compile_time_item_limit() {
    let src = r#"
#[page] fn tags(ctx: PageContext, raw: String, max: i64) -> Result<Json, PageError> {
    let items = splitBounded(raw, ",", max);
    return Ok(json(items.len()));
}
route tags GET "/tags" query raw<String> max<i64> public => tags;
"#;
    let err = compile_source(src).expect_err("collection bound must be statically visible");
    assert!(err.to_string().contains("integer literal in 1..4096"));
}

#[test]
fn split_bounded_accepts_safe_literal_limit() {
    let src = r#"
#[page] fn tags(ctx: PageContext, raw: String) -> Result<Json, PageError> {
    let items = splitBounded(raw, ",", 32);
    return Ok(json(items.len()));
}
route tags GET "/tags" query raw<String> public => tags;
"#;
    compile_source(src).expect("bounded collection constructor should compile");
}

#[test]
fn nominal_domain_ids_are_not_interchangeable() {
    let src = r#"
type UserId = i64 { range 1 999999; }
type ArticleId = i64 { range 1 999999; }
model User { id: UserId }
#[query] fn loadUser(db: Db, id: UserId) -> Result<Option<User>, DbError> sql {
    SELECT id FROM users WHERE id = :id
}
#[page] fn show(ctx: PageContext, db: Db, articleId: ArticleId) -> Result<Json, PageError> {
    let user = loadUser(db, articleId)?;
    return Ok(json(true));
}
route show GET "/articles/:articleId<ArticleId>" auth user => show;
"#;
    let err =
        compile_source(src).expect_err("ArticleId must not be accepted where UserId is required");
    let msg = err.to_string();
    assert!(msg.contains("SEC-TYPE-001"), "{msg}");
    assert!(msg.contains("UserId"), "{msg}");
    assert!(msg.contains("ArticleId"), "{msg}");
}

#[test]
fn raw_int_is_not_a_nominal_domain_id() {
    let src = r#"
type UserId = i64 { range 1 999999; }
model User { id: UserId }
#[query] fn loadUser(db: Db, id: UserId) -> Result<Option<User>, DbError> sql {
    SELECT id FROM users WHERE id = :id
}
#[page] fn show(ctx: PageContext, db: Db) -> Result<Json, PageError> {
    let user = loadUser(db, 1)?;
    return Ok(json(true));
}
route show GET "/users" auth user => show;
"#;
    let err = compile_source(src).expect_err("i64 literal must not silently become UserId");
    let msg = err.to_string();
    assert!(msg.contains("SEC-TYPE-001"), "{msg}");
    assert!(msg.contains("UserId"), "{msg}");
}

#[test]
fn nominal_string_domain_remains_ergonomic_for_safe_string_builtins() {
    let src = r#"
type Username = String {
    length 3 32;
    pattern "^[A-Za-z0-9_]+$";
}
#[page] fn profile(ctx: PageContext, username: Username) -> Result<Html, PageError> {
    let size = username.chars().count();
    return Ok(html { <p>{{ username }} {{ size }}</p> });
}
route profile GET "/profiles/:username<Username>" public => profile;
"#;
    compile_source(src)
        .expect("nominal String domains should work with safe representation-consuming builtins");
}

#[test]
fn explicit_domain_validation_constructs_nominal_value() {
    let src = r#"
type UserId = i64 { range 1 999999; }
model User { id: UserId }
#[query] fn loadUser(db: Db, id: UserId) -> Result<Option<User>, DbError> sql {
    SELECT id FROM users WHERE id = :id
}
#[page] fn show(ctx: PageContext, db: Db, raw: i64) -> Result<Json, PageError> {
    let userId = validate UserId(raw)?;
    let user = loadUser(db, userId)?;
    return Ok(json(true));
}
route show GET "/users" query raw<i64> public => show;
"#;
    compile_source(src).expect("validated primitive should become its nominal domain type");
}

#[test]
fn domain_validation_rejects_wrong_representation() {
    let src = r#"
type UserId = i64 { range 1 999999; }
#[page] fn show(ctx: PageContext, raw: String) -> Result<Json, PageError> {
    let userId = validate UserId(raw)?;
    return Ok(json(true));
}
route show GET "/users" query raw<String> public => show;
"#;
    let err = compile_source(src).expect_err("String must not be refinable into i64-backed UserId");
    let msg = err.to_string();
    assert!(msg.contains("cannot validate `UserId`"), "{msg}");
}

#[test]
fn domain_validation_requires_explicit_failure_propagation() {
    let src = r#"
type UserId = i64 { range 1 999999; }
#[page] fn show(ctx: PageContext, raw: i64) -> Result<Json, PageError> {
    let userId = validate UserId(raw);
    return Ok(json(true));
}
route show GET "/users" query raw<i64> public => show;
"#;
    let err = compile_source(src).expect_err("domain validation must be fail-closed");
    assert!(err.to_string().contains("validate Type(expression)?"));
}

#[test]
fn domain_validation_cannot_relabel_another_nominal_domain() {
    let src = r#"
type UserId = i64 { range 1 999999; }
type ArticleId = i64 { range 1 999999; }
#[page] fn show(ctx: PageContext, articleId: ArticleId) -> Result<Json, PageError> {
    let userId = validate UserId(articleId)?;
    return Ok(json(true));
}
route show GET "/articles/:articleId<ArticleId>" public => show;
"#;
    let err =
        compile_source(src).expect_err("nominal domains must not be re-labelled by validation");
    let msg = err.to_string();
    assert!(msg.contains("SEC-TYPE-002"), "{msg}");
    assert!(msg.contains("ArticleId"), "{msg}");
    assert!(msg.contains("UserId"), "{msg}");
}

#[test]
fn explicit_public_projection_allows_public_fields() {
    let src = r#"
model User {
    id: i64
    displayName: String
    passwordHash: Secret<String>
}
#[query] fn loadUser(db: Db, id: i64) -> Result<User, DbError> sql {
    SELECT id, displayName, passwordHash FROM users WHERE id = :id
}
#[page] fn show(ctx: PageContext, db: Db, id: i64) -> Result<Json, PageError> {
    let user = loadUser(db, id)?;
    return Ok(json(expose(user, id, displayName)));
}
route show GET "/users/:id<i64>" auth user => show;
"#;
    compile_source(src).expect("explicit public projection should compile");
}

#[test]
fn public_projection_rejects_secret_fields() {
    let src = r#"
model User {
    id: i64
    passwordHash: Secret<String>
}
#[query] fn loadUser(db: Db, id: i64) -> Result<User, DbError> sql {
    SELECT id, passwordHash FROM users WHERE id = :id
}
#[page] fn show(ctx: PageContext, db: Db, id: i64) -> Result<Json, PageError> {
    let user = loadUser(db, id)?;
    return Ok(json(expose(user, id, passwordHash)));
}
route show GET "/users/:id<i64>" auth user => show;
"#;
    let err = compile_source(src).expect_err("Secret fields must never be exposed");
    assert!(err.to_string().contains("SEC-DATA-005"));
}

#[test]
fn public_projection_requires_authorization_for_sensitive_fields() {
    let src = r#"
model User {
    id: i64
    email: Sensitive<Email>
    owner: String
}
#[query] fn loadUser(db: Db, id: i64) -> Result<User, DbError> sql {
    SELECT id, email, owner FROM users WHERE id = :id
}
#[page] fn show(ctx: PageContext, db: Db, id: i64) -> Result<Json, PageError> {
    let user = loadUser(db, id)?;
    return Ok(json(expose(user, id, email)));
}
route show GET "/users/:id<i64>" auth user => show;
"#;
    let err = compile_source(src).expect_err("Sensitive fields need authorization proof");
    assert!(err.to_string().contains("SEC-A01-012"));
}

#[test]
fn public_projection_allows_authorized_sensitive_fields() {
    let src = r#"
model User {
    id: i64
    email: Sensitive<Email>
    owner: String
}
#[query] fn loadUser(db: Db, id: i64) -> Result<User, DbError> sql {
    SELECT id, email, owner FROM users WHERE id = :id
}
#[page] fn show(ctx: PageContext, db: Db, id: i64) -> Result<Json, PageError> {
    let user = loadUser(db, id)?;
    authorize user owner owner;
    return Ok(json(expose(user, id, email)));
}
route show GET "/users/:id<i64>" auth user => show;
"#;
    compile_source(src).expect("authorized Sensitive field should be exposable");
}

#[test]
fn direct_model_json_serialization_is_rejected() {
    let src = r#"
model User {
    id: i64
    displayName: String
}
#[query] fn loadUser(db: Db, id: i64) -> Result<User, DbError> sql {
    SELECT id, displayName FROM users WHERE id = :id
}
#[page] fn show(ctx: PageContext, db: Db, id: i64) -> Result<Json, PageError> {
    let user = loadUser(db, id)?;
    return Ok(json(user));
}
route show GET "/users/:id<i64>" auth user => show;
"#;
    let err = compile_source(src).expect_err("models require explicit public projection");
    assert!(err.to_string().contains("SEC-DATA-004"));
}

#[test]
fn public_projection_supports_model_lists() {
    let src = r#"
model Product {
    id: i64
    name: String
}
#[query] fn listProducts(db: Db) -> Result<Vec<Product>, DbError> sql {
    SELECT id, name FROM products LIMIT 100
}
#[page] fn index(ctx: PageContext, db: Db) -> Result<Json, PageError> {
    let products = listProducts(db)?;
    return Ok(json(expose(products, id, name)));
}
route index GET "/products" public => index;
"#;
    compile_source(src).expect("public list projection should compile");
}

#[test]
fn secret_can_flow_only_into_explicit_secret_query_parameter() {
    let src = r#"
model Credential {
    id: i64
    token: Secret<String>
}
#[query] fn loadCredential(db: Db, id: i64) -> Result<Credential, DbError> sql {
    SELECT id, token FROM credentials WHERE id = :id
}
#[query] fn lookupByToken(db: Db, token: Secret<String>) -> Result<Option<Credential>, DbError> sql {
    SELECT id, token FROM credentials WHERE token = :token
}
#[page] fn check(ctx: PageContext, db: Db, id: i64) -> Result<Json, PageError> {
    let credential = loadCredential(db, id)?;
    let found = lookupByToken(db, credential.token)?;
    return Ok(json(true));
}
route check GET "/credentials/:id<i64>" auth user => check;
"#;
    compile_source(src).expect("an explicit Secret<T> query sink should consume secret data");
}

#[test]
fn secret_cannot_flow_into_public_query_parameter() {
    let src = r#"
model Credential {
    id: i64
    token: Secret<String>
}
#[query] fn loadCredential(db: Db, id: i64) -> Result<Credential, DbError> sql {
    SELECT id, token FROM credentials WHERE id = :id
}
#[query] fn unsafeLookup(db: Db, token: String) -> Result<Option<Credential>, DbError> sql {
    SELECT id, token FROM credentials WHERE token = :token
}
#[page] fn check(ctx: PageContext, db: Db, id: i64) -> Result<Json, PageError> {
    let credential = loadCredential(db, id)?;
    let found = unsafeLookup(db, credential.token)?;
    return Ok(json(true));
}
route check GET "/credentials/:id<i64>" auth user => check;
"#;
    let err = compile_source(src).expect_err("Secret<T> must require an explicit secret sink");
    assert!(err.to_string().contains("SEC-DATA-006"), "{err}");
}

#[test]
fn secret_query_parameter_rejects_public_data() {
    let src = r#"
model Credential {
    id: i64
    token: Secret<String>
}
#[query] fn lookupByToken(db: Db, token: Secret<String>) -> Result<Option<Credential>, DbError> sql {
    SELECT id, token FROM credentials WHERE token = :token
}
#[page] fn check(ctx: PageContext, db: Db, token: String) -> Result<Json, PageError> {
    let found = lookupByToken(db, token)?;
    return Ok(json(true));
}
route check GET "/credentials" query token<String> auth user => check;
"#;
    let err =
        compile_source(src).expect_err("public request data must not masquerade as Secret<T>");
    assert!(err.to_string().contains("SEC-DATA-007"), "{err}");
}

#[test]
fn business_audit_rejects_secret_values() {
    let src = r#"
model Account {
    id: i64
    owner: String
    apiKey: Secret<String>
}
#[query] fn loadAccount(db: Db, id: i64) -> Result<Account, DbError> sql {
    SELECT id, owner, apiKey FROM accounts WHERE id = :id
}
#[query] fn touch(tx: Transaction, id: i64) -> Result<Changed, DbError> mutates Account by id sql {
    UPDATE accounts SET owner = owner WHERE id = :id
}
#[action] fn update(ctx: ActionContext, db: Db, id: i64) -> Result<Json, PageError> {
    let account = loadAccount(db, id)?;
    authorize account owner owner;
    transaction db {
        touch(tx, account.id)?;
        audit Account account.id action rotate from account.apiKey to account.apiKey;
    }
    return Ok(json(true));
}
route update POST "/accounts/:id<i64>" auth user => update;
"#;
    let err = compile_source(src).expect_err("business audit must not persist secret values");
    assert!(err.to_string().contains("SEC-DATA-008"), "{err}");
}

#[test]
fn web_handler_cannot_claim_request_parameter_is_secret() {
    let src = r#"
#[page] fn show(ctx: PageContext, token: Secret<String>) -> Result<Json, PageError> {
    return Ok(json(true));
}
route show GET "/show" query token<String> auth user => show;
"#;
    let err = compile_source(src).expect_err("request parameters are not secret capabilities");
    assert!(err.to_string().contains("SEC-DATA-009"), "{err}");
}

#[test]
fn password_hash_produces_secret_data_for_secret_sink() {
    let src = r#"
model Account {
    id: i64
    passwordHash: Secret<PasswordHash>
}
#[query] fn storeHash(tx: Transaction, id: i64, hash: Secret<PasswordHash>) -> Result<(), DbError> mutates Account by id sql {
    UPDATE accounts SET passwordHash = :hash WHERE id = :id
}
#[query] fn loadAccount(db: Db, id: i64) -> Result<Account, DbError> sql {
    SELECT id, passwordHash FROM accounts WHERE id = :id
}
#[action] fn change(ctx: ActionContext, db: Db, id: i64, password: Password) -> Result<Redirect, PageError> {
    let account = loadAccount(db, id)?;
    authorize account authenticated;
    let hash = passwordHash(password);
    transaction db {
        storeHash(tx, account.id, hash)?;
    }
    return Ok(redirect(home()));
}
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> { return Ok(html { <p>ok</p> }); }
route home GET "/" auth user => home;
route change POST "/accounts/:id<i64>/password" form password<Password> auth user => change;
"#;
    compile_source(src).expect(
        "passwordHash should produce Secret<PasswordHash> accepted by a purpose-typed secret sink",
    );
}

#[test]
fn password_hash_cannot_flow_into_public_sink() {
    let src = r#"
#[query] fn unsafeStore(tx: Transaction, hash: String) -> Result<(), DbError> sql {
    INSERT INTO hashes(value) VALUES(:hash)
}
#[action] fn change(ctx: ActionContext, db: Db, password: Password) -> Result<Redirect, PageError> {
    let hash = passwordHash(password);
    transaction db { unsafeStore(tx, hash)?; }
    return Ok(redirect(home()));
}
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> { return Ok(html { <p>ok</p> }); }
route home GET "/" auth user => home;
route change POST "/password" form password<Password> auth user => change;
"#;
    let err = compile_source(src).expect_err("password hashes must remain secret at sinks");
    assert!(err.to_string().contains("SEC-DATA-006"), "{err}");
}

#[test]
fn password_verify_requires_secret_hash() {
    let src = r#"
#[page] fn check(ctx: PageContext, hash: String, password: Password) -> Result<Json, PageError> {
    let valid = passwordVerify(hash, password);
    return Ok(json(valid));
}
route check POST "/check" form hash<String> password<Password> auth user => check;
"#;
    let err =
        compile_source(src).expect_err("passwordVerify must require a Secret<PasswordHash> hash");
    assert!(err.to_string().contains("SEC-A04-001"), "{err}");
}

#[test]
fn password_hash_rejects_plain_string_even_when_validated() {
    let src = r#"
#[page] fn hash(ctx: PageContext, password: String) -> Result<Json, PageError> {
    let value = passwordHash(password);
    return Ok(json(true));
}
route hash POST "/hash" form password<String> public => hash;
"#;
    let err =
        compile_source(src).expect_err("credential primitives require purpose-typed Password");
    assert!(err.to_string().contains("SEC-A04-003"), "{err}");
}

#[test]
fn password_verify_rejects_other_secret_credential_purpose() {
    let src = r#"
model Credential {
    id: i64
    token: Secret<ApiToken>
}
#[query] fn loadCredential(db: Db, id: i64) -> Result<Credential, DbError> sql {
    SELECT id, token FROM credentials WHERE id = :id
}
#[page] fn check(ctx: PageContext, db: Db, id: i64, password: Password) -> Result<Json, PageError> {
    let credential = loadCredential(db, id)?;
    let valid = passwordVerify(credential.token, password);
    return Ok(json(valid));
}
route check POST "/check/:id<i64>" form password<Password> auth user => check;
"#;
    let err = compile_source(src).expect_err("ApiToken must not be usable as PasswordHash");
    assert!(err.to_string().contains("SEC-A04-001"), "{err}");
}

#[test]
fn credential_purpose_value_cannot_be_generically_rendered() {
    let src = r#"
#[page] fn show(ctx: PageContext, password: Password) -> Result<Json, PageError> {
    return Ok(json(password));
}
route show POST "/show" form password<Password> public => show;
"#;
    let err = compile_source(src).expect_err("credentials require dedicated protocol boundaries");
    assert!(err.to_string().contains("SEC-A04-004"), "{err}");
}

#[test]
fn session_token_generation_flows_only_into_matching_secret_sink() {
    let src = r#"
#[query] fn storeSession(tx: Transaction, token: Secret<SessionTokenHash>) -> Result<(), DbError> sql {
    INSERT INTO sessions(token_hash) VALUES(:token)
}
#[action] fn create(ctx: ActionContext, db: Db) -> Result<Redirect, PageError> {
    let token = newSessionToken();
    let storedHash = tokenHash(token);
    transaction db { storeSession(tx, storedHash)?; }
    return Ok(redirect(home()));
}
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> { return Ok(html { <p>ok</p> }); }
route home GET "/" auth user => home;
route create POST "/session" auth user => create;
"#;
    compile_source(src).expect("issued session tokens should hash into Secret<SessionTokenHash>");
}

#[test]
fn token_active_accepts_matching_session_token_with_expiry() {
    let src = r#"
model Session {
    id: i64
    tokenHash: Secret<SessionTokenHash>
    expiresAt: DateTime
}
#[query] fn loadSession(db: Db, id: i64) -> Result<Session, DbError> sql {
    SELECT id, token_hash AS tokenHash, expires_at AS expiresAt FROM sessions WHERE id = :id
}
#[action] fn check(ctx: ActionContext, db: Db, id: i64, token: SessionToken) -> Result<Json, PageError> {
    let session = loadSession(db, id)?;
    let valid = tokenActive(session.tokenHash, token, session.expiresAt);
    return Ok(json(valid));
}
route check POST "/sessions/:id<i64>/check" form token<SessionToken> auth user => check;
"#;
    compile_source(src).expect("session verification should require and accept an explicit expiry");
}

#[test]
fn token_matches_rejects_cross_purpose_comparison() {
    let src = r#"
model Reset {
    id: i64
    tokenHash: Secret<PasswordResetTokenHash>
    expiresAt: DateTime
}
#[query] fn loadReset(db: Db, id: i64) -> Result<Reset, DbError> sql {
    SELECT id, token_hash AS tokenHash, expires_at AS expiresAt FROM resets WHERE id = :id
}
#[page] fn check(ctx: PageContext, db: Db, id: i64, token: SessionToken) -> Result<Json, PageError> {
    let reset = loadReset(db, id)?;
    let valid = tokenActive(reset.tokenHash, token, reset.expiresAt);
    return Ok(json(valid));
}
route check POST "/reset/:id<i64>/check" form token<SessionToken> auth user => check;
"#;
    let err =
        compile_source(src).expect_err("reset and session tokens must never be interchangeable");
    assert!(err.to_string().contains("SEC-A07-001"), "{err}");
}

#[test]
fn token_hash_rejects_presented_request_token() {
    let src = r#"
#[page] fn check(ctx: PageContext, token: SessionToken) -> Result<Json, PageError> {
    let hashed = tokenHash(token);
    return Ok(json(true));
}
route check POST "/check" form token<SessionToken> auth user => check;
"#;
    let err = compile_source(src)
        .expect_err("request-presented tokens must not be promoted into stored token hashes");
    assert!(err.to_string().contains("SEC-A07-003"), "{err}");
}

#[test]
fn token_matches_rejects_raw_stored_token() {
    let src = r#"
model Session {
    id: i64
    token: Secret<SessionToken>
    expiresAt: DateTime
}
#[query] fn loadSession(db: Db, id: i64) -> Result<Session, DbError> sql {
    SELECT id, token, expires_at AS expiresAt FROM sessions WHERE id = :id
}
#[page] fn check(ctx: PageContext, db: Db, id: i64, token: SessionToken) -> Result<Json, PageError> {
    let session = loadSession(db, id)?;
    let valid = tokenActive(session.token, token, session.expiresAt);
    return Ok(json(valid));
}
route check POST "/check/:id<i64>" form token<SessionToken> auth user => check;
"#;
    let err = compile_source(src).expect_err("tokenActive must require an at-rest token hash");
    assert!(err.to_string().contains("SEC-A07-001"), "{err}");
}

#[test]
fn generated_token_cannot_enter_generic_response() {
    let src = r#"
#[page] fn show(ctx: PageContext) -> Result<Json, PageError> {
    let token = newPasswordResetToken();
    return Ok(json(token));
}
route show GET "/show" auth user => show;
"#;
    let err =
        compile_source(src).expect_err("issued credentials require a dedicated delivery boundary");
    assert!(
        err.to_string().contains("SEC-A04-004") || err.to_string().contains("SEC-DATA-001"),
        "{err}"
    );
}

#[test]
fn session_token_cannot_use_expiry_free_token_matches() {
    let src = r#"
model Session {
    id: i64
    tokenHash: Secret<SessionTokenHash>
}
#[query] fn loadSession(db: Db, id: i64) -> Result<Session, DbError> sql {
    SELECT id, token_hash AS tokenHash FROM sessions WHERE id = :id
}
#[page] fn check(ctx: PageContext, db: Db, id: i64, token: SessionToken) -> Result<Json, PageError> {
    let session = loadSession(db, id)?;
    let valid = tokenMatches(session.tokenHash, token);
    return Ok(json(valid));
}
route check POST "/sessions/:id<i64>/check" form token<SessionToken> auth user => check;
"#;
    let err = compile_source(src).expect_err("session verification must include expiry");
    assert!(err.to_string().contains("SEC-A07-004"), "{err}");
}

#[test]
fn csrf_token_matches_remains_expiry_free() {
    let src = r#"
model FormState {
    id: i64
    tokenHash: Secret<CsrfTokenHash>
}
#[query] fn loadState(db: Db, id: i64) -> Result<FormState, DbError> sql {
    SELECT id, token_hash AS tokenHash FROM form_state WHERE id = :id
}
#[action] fn check(ctx: ActionContext, db: Db, id: i64, token: CsrfToken) -> Result<Json, PageError> {
    let state = loadState(db, id)?;
    let valid = tokenMatches(state.tokenHash, token);
    return Ok(json(valid));
}
route check POST "/csrf/:id<i64>/check" form token<CsrfToken> auth user => check;
"#;
    compile_source(src)
        .expect("CSRF tokens may use purpose-safe equality without independent expiry");
}

#[test]
fn password_reset_can_be_consumed_atomically_with_presented_token_proof() {
    let src = r#"
model ResetGrant {
    id: i64
    tokenHash: Secret<PasswordResetTokenHash>
    expiresAt: DateTime
}
#[query] fn consumeReset(tx: Transaction, tokenHash: PasswordResetTokenHash) -> Result<Changed, DbError> consumes ResetGrant by tokenHash before expiresAt sql {
    DELETE FROM reset_grants WHERE token_hash = :tokenHash AND expires_at > CURRENT_TIMESTAMP
}
#[action] fn reset(ctx: ActionContext, db: Db, token: PasswordResetToken) -> Result<Json, PageError> {
    let lookupHash = presentedTokenHash(token);
    transaction db {
        let consumed = consumeReset(tx, lookupHash)?;
    }
    return Ok(json(true));
}
route reset POST "/reset" form token<PasswordResetToken> public => reset;
"#;
    compile_source(src)
        .expect("password-reset token consumption should be atomic and proof-carrying");
}

#[test]
fn password_reset_consume_requires_presented_token_hash_evidence() {
    let src = r#"
model ResetGrant {
    id: i64
    tokenHash: Secret<PasswordResetTokenHash>
    expiresAt: DateTime
}
model ImportedHash {
    id: i64
    tokenHash: PasswordResetTokenHash
}
#[query] fn loadImported(db: Db, id: i64) -> Result<ImportedHash, DbError> sql {
    SELECT id, token_hash AS tokenHash FROM imported_hashes WHERE id = :id
}
#[query] fn consumeReset(tx: Transaction, tokenHash: PasswordResetTokenHash) -> Result<Changed, DbError> consumes ResetGrant by tokenHash before expiresAt sql {
    DELETE FROM reset_grants WHERE token_hash = :tokenHash AND expires_at > CURRENT_TIMESTAMP
}
#[action] fn reset(ctx: ActionContext, db: Db, id: i64) -> Result<Json, PageError> {
    let imported = loadImported(db, id)?;
    transaction db {
        let consumed = consumeReset(tx, imported.tokenHash)?;
    }
    return Ok(json(true));
}
route reset POST "/reset/:id<i64>" public => reset;
"#;
    let err = compile_source(src).expect_err(
        "same-purpose hashes without presented-token evidence must not authorize reset consumption",
    );
    assert!(err.to_string().contains("SEC-A07-015"), "{err}");
}

#[test]
fn password_reset_consume_requires_expiry_predicate_and_changed() {
    let src = r#"
model ResetGrant {
    id: i64
    tokenHash: Secret<PasswordResetTokenHash>
    expiresAt: DateTime
}
#[query] fn consumeReset(tx: Transaction, tokenHash: PasswordResetTokenHash) -> Result<(), DbError> consumes ResetGrant by tokenHash before expiresAt sql {
    DELETE FROM reset_grants WHERE token_hash = :tokenHash
}
#[action] fn reset(ctx: ActionContext, db: Db, token: PasswordResetToken) -> Result<Json, PageError> {
    let lookupHash = presentedTokenHash(token);
    transaction db {
        consumeReset(tx, lookupHash)?;
    }
    return Ok(json(true));
}
route reset POST "/reset" form token<PasswordResetToken> public => reset;
"#;
    let err = compile_source(src)
        .expect_err("reset consumption must use Changed and an expiry predicate");
    assert!(
        err.to_string().contains("SEC-A07-008") || err.to_string().contains("SEC-A07-013"),
        "{err}"
    );
}

#[test]
fn current_session_can_be_revoked_atomically_by_presented_token() {
    let src = r#"
model Session {
    id: i64
    tokenHash: Secret<SessionTokenHash>
}
#[query] fn revokeSession(tx: Transaction, tokenHash: SessionTokenHash) -> Result<Changed, DbError> revokes Session by tokenHash sql {
    DELETE FROM sessions WHERE token_hash = :tokenHash
}
#[action] fn logout(ctx: ActionContext, db: Db, token: SessionToken) -> Result<Json, PageError> {
    let lookupHash = presentedTokenHash(token);
    transaction db {
        let revoked = revokeSession(tx, lookupHash)?;
    }
    return Ok(json(true));
}
route logout POST "/logout" form token<SessionToken> auth user => logout;
"#;
    compile_source(src)
        .expect("current-session revocation should be a one-row atomic lifecycle mutation");
}

#[test]
fn session_can_rotate_atomically_with_presented_and_fresh_token_proofs() {
    let src = r#"
model Session {
    id: i64
    tokenHash: Secret<SessionTokenHash>
    expiresAt: DateTime
}
#[query] fn rotateSession(
    tx: Transaction,
    tokenHash: SessionTokenHash,
    newTokenHash: Secret<SessionTokenHash>,
    expiresAt: DateTime
) -> Result<Changed, DbError> rotates Session by tokenHash to newTokenHash until expiresAt sql {
    UPDATE sessions
    SET token_hash = :newTokenHash, expires_at = :expiresAt
    WHERE token_hash = :tokenHash AND expires_at > CURRENT_TIMESTAMP
}
#[action] fn rotate(ctx: ActionContext, db: Db, token: SessionToken, expiresAt: DateTime) -> Result<Json, PageError> {
    let oldHash = presentedTokenHash(token);
    let newToken = newSessionToken();
    let newHash = tokenHash(newToken);
    transaction db {
        let changed = rotateSession(tx, oldHash, newHash, expiresAt)?;
    }
    return Ok(json(true));
}
route rotate POST "/session/rotate" form token<SessionToken> expiresAt<DateTime> auth user => rotate;
"#;
    compile_source(src)
        .expect("session rotation should require old presented proof and fresh replacement proof");
}

#[test]
fn session_rotation_rejects_reused_stored_hash_as_replacement() {
    let src = r#"
model Session {
    id: i64
    tokenHash: Secret<SessionTokenHash>
    expiresAt: DateTime
}
#[query] fn loadSession(db: Db, id: i64) -> Result<Session, DbError> sql {
    SELECT id, token_hash AS tokenHash, expires_at AS expiresAt FROM sessions WHERE id = :id
}
#[query] fn rotateSession(
    tx: Transaction,
    tokenHash: SessionTokenHash,
    newTokenHash: Secret<SessionTokenHash>,
    expiresAt: DateTime
) -> Result<Changed, DbError> rotates Session by tokenHash to newTokenHash until expiresAt sql {
    UPDATE sessions
    SET token_hash = :newTokenHash, expires_at = :expiresAt
    WHERE token_hash = :tokenHash AND expires_at > CURRENT_TIMESTAMP
}
#[action] fn rotate(ctx: ActionContext, db: Db, id: i64, token: SessionToken, expiresAt: DateTime) -> Result<Json, PageError> {
    let session = loadSession(db, id)?;
    let oldHash = presentedTokenHash(token);
    transaction db {
        let changed = rotateSession(tx, oldHash, session.tokenHash, expiresAt)?;
    }
    return Ok(json(true));
}
route rotate POST "/session/:id<i64>/rotate" form token<SessionToken> expiresAt<DateTime> auth user => rotate;
"#;
    let err = compile_source(src)
        .expect_err("rotation replacement must come from a freshly issued token");
    assert!(err.to_string().contains("SEC-A07-018"), "{err}");
}

#[test]
fn session_rotation_requires_guarded_update_assignments() {
    let src = r#"
model Session {
    id: i64
    tokenHash: Secret<SessionTokenHash>
    expiresAt: DateTime
}
#[query] fn rotateSession(
    tx: Transaction,
    tokenHash: SessionTokenHash,
    newTokenHash: Secret<SessionTokenHash>,
    expiresAt: DateTime
) -> Result<Changed, DbError> rotates Session by tokenHash to newTokenHash until expiresAt sql {
    UPDATE sessions
    SET expires_at = :expiresAt,
        token_hash = token_hash || :newTokenHash
    WHERE token_hash = :tokenHash
}
#[action] fn rotate(ctx: ActionContext, db: Db, token: SessionToken, expiresAt: DateTime) -> Result<Json, PageError> {
    let oldHash = presentedTokenHash(token);
    let newToken = newSessionToken();
    let newHash = tokenHash(newToken);
    transaction db {
        let changed = rotateSession(tx, oldHash, newHash, expiresAt)?;
    }
    return Ok(json(true));
}
route rotate POST "/session/rotate" form token<SessionToken> expiresAt<DateTime> auth user => rotate;
"#;
    let err = compile_source(src)
        .expect_err("rotation must replace hash, set expiry, and guard current expiry atomically");
    assert!(err.to_string().contains("SEC-A07-013"), "{err}");
}

#[test]
fn token_hash_requires_freshly_issued_token_evidence() {
    let src = r#"
model ImportedSessionSecret {
    id: i64
    token: Secret<SessionToken>
}
#[query] fn loadImported(db: Db, id: i64) -> Result<ImportedSessionSecret, DbError> sql {
    SELECT id, token FROM imported_session_secrets WHERE id = :id
}
#[query] fn storeHash(tx: Transaction, tokenHash: Secret<SessionTokenHash>) -> Result<(), DbError> sql {
    INSERT INTO session_hashes(token_hash) VALUES(:tokenHash)
}
#[action] fn persist(ctx: ActionContext, db: Db, id: i64) -> Result<Json, PageError> {
    let imported = loadImported(db, id)?;
    let hash = tokenHash(imported.token);
    transaction db {
        storeHash(tx, hash)?;
    }
    return Ok(json(true));
}
route persist POST "/session/:id<i64>/persist" auth user => persist;
"#;
    let err =
        compile_source(src).expect_err("tokenHash must accept only freshly issued bearer tokens");
    assert!(err.to_string().contains("SEC-A07-003"), "{err}");
}

#[test]
fn function_level_permission_compiles_with_named_route_permission() {
    let src = r#"
permission UserAdmin {
    role Admin
    role SecurityAdmin
}
#[action] fn removeUser(ctx: ActionContext, id: i64) -> Result<Json, PageError> requires UserAdmin {
    return Ok(json(true));
}
route removeUser POST "/users/:id<i64>/remove" auth permission UserAdmin => removeUser;
"#;
    let program =
        compile_source(src).expect("named permission should secure the handler and route");
    match &program.routes[0].auth {
        RouteAuth::Permission { name, roles } => {
            assert!(name.ends_with("UserAdmin"));
            assert_eq!(
                roles,
                &vec!["Admin".to_string(), "SecurityAdmin".to_string()]
            );
        }
        other => panic!("expected permission auth, got {other:?}"),
    }
}

#[test]
fn function_level_permission_rejects_weak_route_auth() {
    let src = r#"
permission UserAdmin {
    role Admin
}
#[action] fn removeUser(ctx: ActionContext, id: i64) -> Result<Json, PageError> requires UserAdmin {
    return Ok(json(true));
}
route removeUser POST "/users/:id<i64>/remove" auth user => removeUser;
"#;
    let err = compile_source(src).expect_err("authenticated-only route must not satisfy UserAdmin");
    assert!(err.to_string().contains("SEC-A01-022"), "{err}");
}

#[test]
fn function_level_permission_accepts_granting_role_for_migration() {
    let src = r#"
permission UserAdmin {
    role Admin
    role SecurityAdmin
}
#[action] fn removeUser(ctx: ActionContext, id: i64) -> Result<Json, PageError> requires UserAdmin {
    return Ok(json(true));
}
route removeUser POST "/users/:id<i64>/remove" auth role Admin => removeUser;
"#;
    compile_source(src).expect(
        "a role explicitly granting the required permission should satisfy the handler contract",
    );
}

#[test]
fn function_level_permission_rejects_non_granting_role() {
    let src = r#"
permission UserAdmin {
    role Admin
}
#[action] fn removeUser(ctx: ActionContext, id: i64) -> Result<Json, PageError> requires UserAdmin {
    return Ok(json(true));
}
route removeUser POST "/users/:id<i64>/remove" auth role Viewer => removeUser;
"#;
    let err =
        compile_source(src).expect_err("unrelated role must not satisfy the handler permission");
    assert!(err.to_string().contains("SEC-A01-022"), "{err}");
}

#[test]
fn route_rejects_unknown_named_permission() {
    let src = r#"
#[action] fn removeUser(ctx: ActionContext, id: i64) -> Result<Json, PageError> {
    return Ok(json(true));
}
route removeUser POST "/users/:id<i64>/remove" auth permission UserAdmin => removeUser;
"#;
    let err = compile_source(src).expect_err("unknown route permission must fail closed");
    assert!(err.to_string().contains("SEC-A01-021"), "{err}");
}

#[test]
fn mfa_requirement_compiles_with_permission_and_mfa_route() {
    let src = r#"
permission BillingWrite {
    role Admin
    role BillingAdmin
}
#[action] fn refund(ctx: ActionContext, id: i64) -> Result<Json, PageError> requires BillingWrite + mfa {
    return Ok(json(true));
}
route refund POST "/billing/:id<i64>/refund" auth permission BillingWrite mfa => refund;
"#;
    let program = compile_source(src).expect("permission + MFA should secure a privileged handler");
    assert!(matches!(
        program.routes[0].auth,
        RouteAuth::PermissionMfa { .. }
    ));
}

#[test]
fn mfa_requirement_rejects_permission_without_elevation() {
    let src = r#"
permission BillingWrite {
    role BillingAdmin
}
#[action] fn refund(ctx: ActionContext, id: i64) -> Result<Json, PageError> requires BillingWrite + mfa {
    return Ok(json(true));
}
route refund POST "/billing/:id<i64>/refund" auth permission BillingWrite => refund;
"#;
    let err =
        compile_source(src).expect_err("permission alone must not satisfy MFA-elevated handler");
    assert!(err.to_string().contains("SEC-A07-020"), "{err}");
}

#[test]
fn mfa_only_requirement_compiles_with_mfa_route() {
    let src = r#"
#[action] fn disableMfa(ctx: ActionContext) -> Result<Json, PageError> requires mfa {
    return Ok(json(true));
}
route disableMfa POST "/account/mfa/disable" auth mfa => disableMfa;
"#;
    compile_source(src).expect("MFA-only handler should compile behind auth mfa");
}

#[test]
fn mfa_only_requirement_rejects_authenticated_route() {
    let src = r#"
#[action] fn disableMfa(ctx: ActionContext) -> Result<Json, PageError> requires mfa {
    return Ok(json(true));
}
route disableMfa POST "/account/mfa/disable" auth user => disableMfa;
"#;
    let err = compile_source(src).expect_err("authenticated session without MFA must fail closed");
    assert!(err.to_string().contains("SEC-A07-020"), "{err}");
}

#[test]
fn critical_operation_compiles_with_inferred_route_auth_and_required_controls() {
    let src = r#"
permission BillingWrite {
    role Admin
    role BillingAdmin
}
critical Payment {
    permission BillingWrite
    mfa
    transaction
    audit
}
model Payment {
    id: i64
}
#[action] fn refund(ctx: ActionContext, db: Db, id: i64) -> Result<Json, PageError> critical Payment {
    transaction db {
        audit Payment id action refund;
    }
    return Ok(json(true));
}
route refund POST "/billing/:id<i64>/refund" auth critical Payment => refund;
"#;
    let program = compile_source(src)
        .expect("critical contract should compose permission, MFA, transaction and audit");
    assert!(matches!(
        program.routes[0].auth,
        RouteAuth::PermissionMfa { .. }
    ));
    let action = program.action("refund").expect("action");
    assert!(action.security.mfa_required);
    assert!(
        action
            .security
            .required_permission
            .as_deref()
            .is_some_and(|value| value.ends_with("BillingWrite"))
    );
    assert!(
        action
            .security
            .critical_operation
            .as_deref()
            .is_some_and(|value| value.ends_with("Payment"))
    );
}

#[test]
fn critical_operation_rejects_missing_transaction() {
    let src = r#"
critical AccountRecovery {
    transaction
}
#[action] fn recover(ctx: ActionContext) -> Result<Json, PageError> critical AccountRecovery {
    return Ok(json(true));
}
route recover POST "/recover" auth critical AccountRecovery => recover;
"#;
    let err = compile_source(src).expect_err("critical transaction requirement must fail closed");
    assert!(err.to_string().contains("SEC-A06-004"), "{err}");
}

#[test]
fn critical_operation_rejects_missing_audit() {
    let src = r#"
critical RoleChange {
    audit
}
#[action] fn grant(ctx: ActionContext) -> Result<Json, PageError> critical RoleChange {
    return Ok(json(true));
}
route grant POST "/roles/grant" auth critical RoleChange => grant;
"#;
    let err = compile_source(src).expect_err("critical audit requirement must fail closed");
    assert!(err.to_string().contains("SEC-A09-001"), "{err}");
}

#[test]
fn critical_operation_rejects_weaker_explicit_route_auth() {
    let src = r#"
permission BillingWrite {
    role BillingAdmin
}
critical Payment {
    permission BillingWrite
    mfa
}
#[action] fn refund(ctx: ActionContext) -> Result<Json, PageError> critical Payment {
    return Ok(json(true));
}
route refund POST "/refund" auth permission BillingWrite => refund;
"#;
    let err = compile_source(src)
        .expect_err("critical operation MFA requirement must still be enforced with explicit auth");
    assert!(err.to_string().contains("SEC-A07-020"), "{err}");
}

#[test]
fn external_string_list_gets_implicit_cardinality_bound() {
    let src = r#"
#[page] fn tags(ctx: PageContext, tags: Vec<String>) -> Result<Json, PageError> {
    return Ok(json(tags.len()));
}
route tags GET "/tags" query tags<Vec<String>> public => tags;
"#;
    let program = compile_source(src).expect("bounded external string list should compile");
    let route = &program.routes[0];
    assert!(route.validations.iter().any(|rule| {
        rule.field == "tags" && matches!(&rule.kind, ValidationKind::Items { min: 1, max: 64 })
    }));
}

#[test]
fn explicit_items_bound_overrides_collection_default() {
    let src = r#"
#[page] fn tags(ctx: PageContext, tags: Vec<String>) -> Result<Json, PageError> {
    return Ok(json(tags.len()));
}
route tags GET "/tags" query tags<Vec<String>> validate tags items 1 8 public => tags;
"#;
    let program = compile_source(src).expect("explicit collection bound should compile");
    let rules: Vec<_> = program.routes[0]
        .validations
        .iter()
        .filter(|rule| rule.field == "tags" && matches!(&rule.kind, ValidationKind::Items { .. }))
        .collect();
    assert_eq!(rules.len(), 1);
    assert!(matches!(
        rules[0].kind,
        ValidationKind::Items { min: 1, max: 8 }
    ));
}

#[test]
fn items_validation_rejects_non_collection_field() {
    let src = r#"
#[page] fn search(ctx: PageContext, q: String) -> Result<Json, PageError> {
    return Ok(json(q));
}
route search GET "/search" query q<String> validate q items 1 8 public => search;
"#;
    let err = compile_source(src).expect_err("items validation must be collection-only");
    assert!(
        err.to_string()
            .contains("validation kind does not match field")
    );
}

#[test]
fn critical_operation_can_require_named_security_event() {
    let src = r#"
model User { id: i64 }
security event RoleGranted for User;
permission UserAdmin { role Admin }
critical RoleChange {
    permission UserAdmin
    mfa
    transaction
    audit RoleGranted
}
#[query] fn record(tx: Transaction, id: i64) -> Result<(), DbError> sql {
    INSERT INTO role_events(user_id) VALUES(:id)
}
#[action] fn grant(ctx: ActionContext, db: Db, id: i64) -> Result<Json, PageError> critical RoleChange {
    transaction db {
        record(tx, id)?;
        security RoleGranted id;
    }
    return Ok(json(true));
}
route grant POST "/grant/:id<i64>" auth critical RoleChange => grant;
"#;
    compile_source(src).expect("named security event should satisfy critical audit contract");
}

#[test]
fn critical_operation_rejects_wrong_security_event() {
    let src = r#"
model User { id: i64 }
security event RoleGranted for User;
security event RoleRevoked for User;
permission UserAdmin { role Admin }
critical RoleChange {
    permission UserAdmin
    mfa
    transaction
    audit RoleGranted
}
#[query] fn record(tx: Transaction, id: i64) -> Result<(), DbError> sql {
    INSERT INTO role_events(user_id) VALUES(:id)
}
#[action] fn grant(ctx: ActionContext, db: Db, id: i64) -> Result<Json, PageError> critical RoleChange {
    transaction db {
        record(tx, id)?;
        security RoleRevoked id;
    }
    return Ok(json(true));
}
route grant POST "/grant/:id<i64>" auth critical RoleChange => grant;
"#;
    let err = compile_source(src)
        .expect_err("wrong security event must not satisfy critical audit contract");
    assert!(err.to_string().contains("SEC-A09-006"), "{err}");
}

#[test]
fn active_webhook_signing_and_verification_keys_compile() {
    let src = r#"
model KeyRing {
    id: i64
    signing: Secret<SigningKey<Webhook>>
    verification: Secret<VerificationKey<Webhook>>
}
#[query] fn loadKeys(db: Db, id: i64) -> Result<KeyRing, DbError> sql {
    SELECT id, signing, verification FROM key_ring WHERE id = :id
}
#[action] fn sign(ctx: ActionContext, db: Db, id: i64, payload: String) -> Result<Json, PageError> {
    let keys = loadKeys(db, id)?;
    let signature = signWebhook(keys.signing, payload);
    let valid = verifyWebhookSignature(keys.verification, payload, signature);
    return Ok(json(valid));
}
route sign POST "/sign/:id<i64>" json payload<String> auth user => sign;
"#;
    compile_source(src).expect("purpose-typed active webhook keys should compile");
}

#[test]
fn retiring_signing_key_cannot_create_new_signature() {
    let src = r#"
model KeyRing {
    id: i64
    signing: Secret<RetiringSigningKey<Webhook>>
}
#[query] fn loadKeys(db: Db, id: i64) -> Result<KeyRing, DbError> sql {
    SELECT id, signing FROM key_ring WHERE id = :id
}
#[action] fn sign(ctx: ActionContext, db: Db, id: i64, payload: String) -> Result<Json, PageError> {
    let keys = loadKeys(db, id)?;
    let signature = signWebhook(keys.signing, payload);
    return Ok(json(signature));
}
route sign POST "/sign/:id<i64>" json payload<String> auth user => sign;
"#;
    let err =
        compile_source(src).expect_err("retiring signing keys must not create new signatures");
    assert!(err.to_string().contains("SEC-A04-020"), "{err}");
}

#[test]
fn retiring_verification_key_can_verify_but_retired_key_cannot() {
    let accepted = r#"
model KeyRing {
    id: i64
    verification: Secret<RetiringVerificationKey<Webhook>>
}
#[query] fn loadKeys(db: Db, id: i64) -> Result<KeyRing, DbError> sql {
    SELECT id, verification FROM key_ring WHERE id = :id
}
#[action] fn verify(ctx: ActionContext, db: Db, id: i64, payload: String, signature: String) -> Result<Json, PageError> {
    let keys = loadKeys(db, id)?;
    return Ok(json(verifyWebhookSignature(keys.verification, payload, signature)));
}
route verify POST "/verify/:id<i64>" json payload<String> signature<String> auth user => verify;
"#;
    compile_source(accepted)
        .expect("retiring verification key should remain valid during rotation");

    let rejected = accepted.replace("RetiringVerificationKey", "RetiredVerificationKey");
    let err = compile_source(&rejected).expect_err("retired verification key must be rejected");
    assert!(err.to_string().contains("SEC-A04-021"), "{err}");
}

#[test]
fn web_request_cannot_supply_crypto_key_capability() {
    let src = r#"
#[action] fn bad(ctx: ActionContext, key: SigningKey<Webhook>) -> Result<Json, PageError> {
    return Ok(json(true));
}
route bad POST "/bad" json key<SigningKey<Webhook>> auth user => bad;
"#;
    let err = compile_source(src)
        .expect_err("crypto key material must never be accepted from request input");
    assert!(err.to_string().contains("SEC-A04-023"), "{err}");
}

#[test]
fn active_user_data_encryption_and_decryption_compile() {
    let src = r#"
model Vault {
    id: i64
    encryption: Secret<EncryptionKey<UserData>>
    value: Sensitive<String>
}
#[query] fn loadVault(db: Db, id: i64) -> Result<Vault, DbError> sql {
    SELECT id, encryption, value FROM vault WHERE id = :id
}
#[action] fn protect(ctx: ActionContext, db: Db, id: i64) -> Result<Json, PageError> {
    let vault = loadVault(db, id)?;
    let sealed = encryptUserData(vault.encryption, vault.value);
    let plain = decryptUserData(vault.encryption, sealed);
    return Ok(json(true));
}
route protect POST "/protect/:id<i64>" auth user => protect;
"#;
    compile_source(src).expect("active user-data encryption key should encrypt and decrypt");
}

#[test]
fn retiring_encryption_key_cannot_encrypt_but_can_decrypt() {
    let encrypt = r#"
model Vault {
    id: i64
    encryption: Secret<RetiringEncryptionKey<UserData>>
    value: Sensitive<String>
}
#[query] fn loadVault(db: Db, id: i64) -> Result<Vault, DbError> sql {
    SELECT id, encryption, value FROM vault WHERE id = :id
}
#[action] fn protect(ctx: ActionContext, db: Db, id: i64) -> Result<Json, PageError> {
    let vault = loadVault(db, id)?;
    let sealed = encryptUserData(vault.encryption, vault.value);
    return Ok(json(true));
}
route protect POST "/protect/:id<i64>" auth user => protect;
"#;
    let err = compile_source(encrypt)
        .expect_err("retiring encryption keys must not create new ciphertext");
    assert!(err.to_string().contains("SEC-A04-020"), "{err}");

    let decrypt = r#"
model Vault {
    id: i64
    encryption: Secret<RetiringEncryptionKey<UserData>>
    value: Sensitive<String>
}
#[query] fn loadVault(db: Db, id: i64) -> Result<Vault, DbError> sql {
    SELECT id, encryption, value FROM vault WHERE id = :id
}
#[action] fn read(ctx: ActionContext, db: Db, id: i64) -> Result<Json, PageError> {
    let vault = loadVault(db, id)?;
    let plain = decryptUserData(vault.encryption, vault.value);
    return Ok(json(true));
}
route read POST "/read/:id<i64>" auth user => read;
"#;
    compile_source(decrypt)
        .expect("retiring encryption keys should remain usable for decryption during rotation");
}

#[test]
fn retired_encryption_key_cannot_decrypt() {
    let src = r#"
model Vault {
    id: i64
    encryption: Secret<RetiredEncryptionKey<UserData>>
    value: Sensitive<String>
}
#[query] fn loadVault(db: Db, id: i64) -> Result<Vault, DbError> sql {
    SELECT id, encryption, value FROM vault WHERE id = :id
}
#[action] fn read(ctx: ActionContext, db: Db, id: i64) -> Result<Json, PageError> {
    let vault = loadVault(db, id)?;
    let plain = decryptUserData(vault.encryption, vault.value);
    return Ok(json(true));
}
route read POST "/read/:id<i64>" auth user => read;
"#;
    let err = compile_source(src).expect_err("retired encryption keys must not decrypt");
    assert!(err.to_string().contains("SEC-A04-024"), "{err}");
}
