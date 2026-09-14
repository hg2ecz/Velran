use super::*;

mod m36_modules_slug_compile_tests {
    use super::*;
    use std::collections::HashSet;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_app() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("velran-m36-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn compiles_multi_file_application_and_preserves_source_file() {
        let dir = temp_app();
        fs::write(dir.join("main.vrn"), "mod pages;\n").unwrap();
        fs::write(
            dir.join("pages.vrn"),
            r#"
#[page] fn article(ctx: PageContext, slug: Slug) -> Result<Html, PageError> {
    return Ok(html {<h1>{{ slug }}</h1>});
}
route article GET "/cikk/:slug<Slug>" public => pages::article;
"#,
        )
        .unwrap();
        let program = compile_file(dir.join("main.vrn")).unwrap();
        assert_eq!(program.routes.len(), 1);
        assert_eq!(program.routes[0].segments.len(), 2);
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn compile_file_reports_transitive_source_dependencies() {
        let dir = temp_app();
        fs::create_dir_all(dir.join("pages")).unwrap();
        fs::write(dir.join("main.vrn"), "mod pages;\n").unwrap();
        fs::write(dir.join("pages.vrn"), "mod article;\n").unwrap();
        fs::write(
            dir.join("pages/article.vrn"),
            r#"
#[page] fn article(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {ok}); }
route article GET "/article" public => pages::article::article;
"#,
        )
        .unwrap();
        let compiled = compile_file_with_dependencies(dir.join("main.vrn")).unwrap();
        let names: HashSet<String> = compiled
            .source_files
            .iter()
            .filter_map(|path| path.file_name().and_then(|v| v.to_str()).map(str::to_owned))
            .collect();
        assert!(names.contains("main.vrn"));
        assert!(names.contains("pages.vrn"));
        assert!(names.contains("article.vrn"));
        assert!(compiled.program.page("pages::article::article").is_some());
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn nested_modules_use_application_root_namespace_layout() {
        let dir = temp_app();
        fs::create_dir_all(dir.join("pages")).unwrap();
        fs::write(dir.join("main.vrn"), "mod pages;\n").unwrap();
        fs::write(dir.join("pages.vrn"), "mod article;\n").unwrap();
        fs::write(
            dir.join("pages/article.vrn"),
            r#"
#[page] fn article(ctx: PageContext, slug: Slug) -> Result<Html, PageError> { return Ok(html {ok}); }
route article GET "/cikk/:slug<Slug>" public => pages::article::article;
"#,
        )
        .unwrap();
        let program = compile_file(dir.join("main.vrn")).unwrap();
        assert!(program.page("pages::article::article").is_some());
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn module_resolution_is_fail_closed() {
        let dir = temp_app();
        fs::create_dir_all(dir.join("pages")).unwrap();
        fs::write(dir.join("main.vrn"), "mod pages::article;\n").unwrap();
        fs::write(dir.join("pages/mod.vrn"), "").unwrap();
        let err = compile_file(dir.join("main.vrn")).unwrap_err().to_string();
        assert!(err.contains("pages/article.vrn"));
        assert!(err.contains("declaring module") || err.contains("application-root"));
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn module_symbols_stay_in_their_namespace() {
        let dir = temp_app();
        fs::write(dir.join("main.vrn"), "mod a;\nmod b;\n").unwrap();
        fs::write(
            dir.join("a.vrn"),
            r#"
#[page] fn show(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {a}); }
route showA GET "/a" public => a::show;
"#,
        )
        .unwrap();
        fs::write(
            dir.join("b.vrn"),
            r#"
#[page] fn show(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {b}); }
"#,
        )
        .unwrap();
        let program = compile_file(dir.join("main.vrn")).unwrap();
        assert!(program.page("a::show").is_some());
        assert!(program.page("b::show").is_some());
        assert!(program.page("show").is_none());
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn local_handler_reference_resolves_inside_its_module() {
        let dir = temp_app();
        fs::write(dir.join("main.vrn"), "mod pages;\n").unwrap();
        fs::write(
            dir.join("pages.vrn"),
            r#"
#[page] fn show(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {ok}); }
route show GET "/" public => show;
"#,
        )
        .unwrap();
        let program = compile_file(dir.join("main.vrn")).unwrap();
        assert_eq!(program.routes[0].handler, "pages::show");
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn cross_module_handler_reference_must_be_qualified() {
        let dir = temp_app();
        fs::write(dir.join("main.vrn"), "mod pages;\nmod routes;\n").unwrap();
        fs::write(
            dir.join("pages.vrn"),
            r#"
#[page] fn show(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {ok}); }
"#,
        )
        .unwrap();
        fs::write(
            dir.join("routes.vrn"),
            r#"
route show GET "/" public => show;
"#,
        )
        .unwrap();
        let err = compile_file(dir.join("main.vrn")).unwrap_err().to_string();
        assert!(err.contains("unknown handler") || err.contains("routes::show"));
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn crate_prefix_selects_application_root() {
        let dir = temp_app();
        fs::create_dir_all(dir.join("pages")).unwrap();
        fs::write(dir.join("main.vrn"), "mod pages;\n").unwrap();
        fs::write(dir.join("pages.vrn"), "mod crate::shared;\n").unwrap();
        fs::write(
            dir.join("shared.vrn"),
            "pub fn answer() -> i64 { return 42; }\n",
        )
        .unwrap();
        fs::write(dir.join("pages/extra.vrn"), "").ok();
        let program = compile_file(dir.join("main.vrn")).unwrap_err().to_string();
        assert!(program.contains("no routes declared"));
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn module_declarations_after_code_are_rejected() {
        let dir = temp_app();
        fs::write(
            dir.join("main.vrn"),
            r#"
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {ok}); }
mod pages;
"#,
        )
        .unwrap();
        fs::write(dir.join("pages.vrn"), "").unwrap();
        let err = compile_file(dir.join("main.vrn")).unwrap_err().to_string();
        assert!(err.contains("must appear before"));
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn super_prefix_cannot_escape_application_root() {
        let dir = temp_app();
        fs::write(dir.join("main.vrn"), "mod super::pages;\n").unwrap();
        let err = compile_file(dir.join("main.vrn")).unwrap_err().to_string();
        assert!(err.contains("cannot escape"));
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn use_alias_expands_only_explicit_module_prefixes() {
        let dir = temp_app();
        fs::write(dir.join("main.vrn"), "mod metrics;\nmod pages;\n").unwrap();
        fs::write(
            dir.join("metrics.vrn"),
            "pub fn answer() -> i64 { return 42; }\n",
        )
        .unwrap();
        fs::write(
            dir.join("pages.vrn"),
            r#"
use metrics as m;
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> {
    let value = m::answer();
    return Ok(html {<p>{{ value }}</p>});
}
route home GET "/" public => home;
"#,
        )
        .unwrap();
        let program = compile_file(dir.join("main.vrn")).unwrap();
        assert!(program.page("pages::home").is_some());
        fs::remove_dir_all(dir).ok();
    }
}
