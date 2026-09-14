use super::*;
use language_core::Effect;

#[test]
fn named_integration_declares_network_effect_on_handler_boundary() {
    let source = r#"
integration Stripe {
    egress payments
}

#[action] fn charge(ctx: ActionContext) -> Result<Json, PageError> uses Stripe {
    return Ok(json(true));
}

route charge POST "/charge"
    public
    => charge;
"#;
    let program = compile_source(source).expect("named integration should compile");
    let action = program.action("charge").expect("action");
    assert_eq!(action.effects, vec![Effect::Network("payments".into())]);
    let integration = program.integration("Stripe").expect("integration");
    assert_eq!(integration.egress_target, "payments");
}

#[test]
fn integration_requires_named_egress_target() {
    let source = r#"
integration Stripe {
}

#[page] fn home(ctx: PageContext) -> Result<Json, PageError> {
    return Ok(json(true));
}

route home GET "/" public => home;
"#;
    let error = compile_source(source).expect_err("missing egress target must fail");
    assert!(format!("{error:?}").contains("SEC-SSRF-001"));
}

#[test]
fn handler_cannot_use_unknown_integration() {
    let source = r#"
#[action] fn charge(ctx: ActionContext) -> Result<Json, PageError> uses Stripe {
    return Ok(json(true));
}

route charge POST "/charge" public => charge;
"#;
    let error = compile_source(source).expect_err("unknown integration must fail");
    assert!(format!("{error:?}").contains("SEC-SSRF-003"));
}

#[test]
fn raw_url_is_not_valid_egress_authority() {
    let source = r#"
integration Stripe {
    egress https://api.stripe.com
}

#[page] fn home(ctx: PageContext) -> Result<Json, PageError> {
    return Ok(json(true));
}

route home GET "/" public => home;
"#;
    let error = compile_source(source).expect_err("URLs do not belong in integration authority");
    assert!(format!("{error:?}").contains("SEC-SSRF-002"));
}

#[test]
fn network_effect_merges_with_inferred_database_effects() {
    let source = r#"
model Item {
    id: i64
}
#[query] fn load(db: Db, id: i64) -> Result<Item, DbError> sql {
    SELECT id FROM items WHERE id = :id
}

integration Catalog {
    egress catalog_api
}

#[page] fn show(ctx: PageContext, db: Db, id: i64) -> Result<Json, PageError> uses Catalog {
    let item = load(db, id)?;
    return Ok(json(expose(item, id)));
}

route show GET "/items/:id<i64>" public => show;
"#;
    let program = compile_source(source).expect("effects should merge");
    let page = program.page("show").expect("page");
    assert_eq!(
        page.effects,
        vec![Effect::DbRead, Effect::Network("catalog_api".into())]
    );
}

#[test]
fn typed_outbound_get_requires_explicit_uses_capability() {
    let source = r#"
integration Catalog {
    egress catalog_api
}

#[page] fn show(ctx: PageContext) -> Result<Json, PageError> {
    let status = Catalog.get("/v1/items")?;
    return Ok(json(status));
}

route show GET "/items" public => show;
"#;
    let error = compile_source(source).expect_err("network effect without uses must fail");
    assert!(format!("{error:?}").contains("SEC-EFFECT-002"));
}

#[test]
fn typed_outbound_get_compiles_with_named_capability() {
    let source = r#"
integration Catalog {
    egress catalog_api
}

#[page] fn show(ctx: PageContext) -> Result<Json, PageError> uses Catalog {
    let status = Catalog.get("/v1/items?limit=10")?;
    return Ok(json(status));
}

route show GET "/items" public => show;
"#;
    let program = compile_source(source).expect("typed outbound GET should compile");
    let page = program.page("show").expect("page");
    assert_eq!(page.effects, vec![Effect::Network("catalog_api".into())]);
}

#[test]
fn dynamic_or_absolute_outbound_paths_are_rejected() {
    let dynamic = r#"
integration Catalog { egress catalog_api }
#[page] fn show(ctx: PageContext, path: String) -> Result<Json, PageError> uses Catalog {
    let status = Catalog.get(path)?;
    return Ok(json(status));
}
route show GET "/items" query path<String> public => show;
"#;
    let error = compile_source(dynamic).expect_err("dynamic outbound path must fail");
    assert!(format!("{error:?}").contains("SEC-SSRF-004"));

    let absolute = r#"
integration Catalog { egress catalog_api }
#[page] fn show(ctx: PageContext) -> Result<Json, PageError> uses Catalog {
    let status = Catalog.get("https://evil.example/x")?;
    return Ok(json(status));
}
route show GET "/items" public => show;
"#;
    let error = compile_source(absolute).expect_err("absolute outbound path must fail");
    assert!(format!("{error:?}").contains("SEC-SSRF-005"));
}

#[test]
fn page_cannot_perform_outbound_post_json() {
    let source = r#"
integration Catalog { egress catalog_api }
#[page] fn show(ctx: PageContext) -> Result<Json, PageError> uses Catalog {
    let status = Catalog.postJson("/v1/items", "payload")?;
    return Ok(json(status));
}
route show GET "/items" public => show;
"#;
    let error = compile_source(source).expect_err("page POST effect must fail");
    assert!(format!("{error:?}").contains("SEC-SSRF-006"));
}

#[test]
fn action_can_post_public_json_and_get_status() {
    let source = r#"
integration Billing { egress payments }
#[action] fn charge(ctx: ActionContext, amount: i64) -> Result<Json, PageError> uses Billing {
    let status = Billing.postJson("/v1/charges", amount)?;
    return Ok(json(status));
}
route charge POST "/charge" form amount<i64> public => charge;
"#;
    let program = compile_source(source).expect("typed outbound POST should compile");
    let action = program.action("charge").expect("action");
    assert_eq!(action.effects, vec![Effect::Network("payments".into())]);
}
