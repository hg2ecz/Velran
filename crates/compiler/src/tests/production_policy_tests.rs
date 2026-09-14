use crate::compile_source;

fn app(policy: &str) -> String {
    format!(
        r#"
{policy}
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> {{
    return Ok(html {{ <p>"ok"</p> }});
}}
route home GET "/" public => home;
"#
    )
}

#[test]
fn strict_production_policy_compiles() {
    let program = compile_source(&app(
        "production { https required; debug disabled; hsts required; database tls required; }",
    ))
    .expect("strict production policy should compile");
    assert!(program.production.is_some());
}

#[test]
fn incomplete_production_policy_is_rejected() {
    let err = compile_source(&app("production { https required; debug disabled; }")).unwrap_err();
    assert!(err.to_string().contains("SEC-PROD-002"));
}

#[test]
fn insecure_or_unknown_production_switch_is_rejected() {
    let err = compile_source(&app(
        "production { https optional; debug disabled; hsts required; database tls required; }",
    ))
    .unwrap_err();
    assert!(err.to_string().contains("SEC-PROD-001"));
}
