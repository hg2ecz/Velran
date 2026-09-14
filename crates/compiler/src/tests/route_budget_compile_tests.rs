use super::*;

#[test]
fn route_budget_records_named_profile_use() {
    let src = r#"
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> {
return Ok(html {<p>ok</p>});
}
route home GET "/"
public
budget interactive
=> home;
"#;
    let program = compile_source(src).expect("route budget should compile");
    assert_eq!(
        program.routes[0].budget_profile.as_deref(),
        Some("interactive")
    );
    assert!(program.resource_uses.iter().any(|use_site| {
        use_site.profile == "interactive" && use_site.source.function == "home"
    }));
}

#[test]
fn route_budget_cannot_be_combined_with_scoped_resource_block() {
    let src = r#"
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> {
with resource compute {
    return Ok(html {<p>ok</p>});
}
}
route home GET "/" public budget interactive => home;
"#;
    let error = compile_source(src).expect_err("nested route and handler budgets must fail closed");
    assert!(error.to_string().contains("SEC-A10-001"), "{error}");
}

#[test]
fn explicit_default_route_budget_is_rejected_as_redundant() {
    let src = r#"
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> {
return Ok(html {<p>ok</p>});
}
route home GET "/" public budget default => home;
"#;
    let error = compile_source(src).expect_err("default budget is implicit");
    assert!(error.to_string().contains("SEC-A10-002"), "{error}");
}
