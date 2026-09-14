use super::*;

mod module_visibility_and_reexport_tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_app() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("velran-module-api-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn private_nested_module_is_not_accessible_from_sibling() {
        let dir = temp_app();
        fs::create_dir_all(dir.join("catalog")).unwrap();
        fs::write(dir.join("main.vrn"), "mod catalog;\nmod pages;\n").unwrap();
        fs::write(dir.join("catalog.vrn"), "mod internal;\n").unwrap();
        fs::write(
            dir.join("catalog/internal.vrn"),
            "pub fn answer() -> i64 { return 42; }\n",
        )
        .unwrap();
        fs::write(
            dir.join("pages.vrn"),
            r#"
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> {
    let value = catalog::internal::answer();
    return Ok(html {<p>{{ value }}</p>});
}
route home GET "/" public => home;
"#,
        )
        .unwrap();
        let err = compile_file(dir.join("main.vrn")).unwrap_err().to_string();
        assert!(err.contains("pub mod") || err.contains("not accessible"));
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn public_nested_module_is_accessible_from_sibling() {
        let dir = temp_app();
        fs::create_dir_all(dir.join("catalog")).unwrap();
        fs::write(dir.join("main.vrn"), "mod catalog;\nmod pages;\n").unwrap();
        fs::write(dir.join("catalog.vrn"), "pub mod internal;\n").unwrap();
        fs::write(
            dir.join("catalog/internal.vrn"),
            "pub fn answer() -> i64 { return 42; }\n",
        )
        .unwrap();
        fs::write(
            dir.join("pages.vrn"),
            r#"
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> {
    let value = catalog::internal::answer();
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

    #[test]
    fn package_root_pub_use_reexports_only_public_module_namespace() {
        let dir = temp_app();
        fs::write(
            dir.join("main.vrn"),
            "pub mod catalog;\npub use crate::catalog as api;\nmod pages;\n",
        )
        .unwrap();
        fs::write(
            dir.join("catalog.vrn"),
            "pub fn answer() -> i64 { return 42; }\n",
        )
        .unwrap();
        fs::write(
            dir.join("pages.vrn"),
            r#"
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> {
    let value = api::answer();
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

    #[test]
    fn pub_use_cannot_reexport_private_module() {
        let dir = temp_app();
        fs::write(
            dir.join("main.vrn"),
            "mod catalog;\npub use crate::catalog as api;\n",
        )
        .unwrap();
        fs::write(
            dir.join("catalog.vrn"),
            "pub fn answer() -> i64 { return 42; }\n",
        )
        .unwrap();
        let err = compile_file(dir.join("main.vrn")).unwrap_err().to_string();
        assert!(err.contains("crosses private module"), "{err}");
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn pub_use_cannot_reexport_item_directly() {
        let dir = temp_app();
        fs::write(
            dir.join("main.vrn"),
            "pub mod catalog;\npub use crate::catalog::answer as answer;\n",
        )
        .unwrap();
        fs::write(
            dir.join("catalog.vrn"),
            "pub fn answer() -> i64 { return 42; }\n",
        )
        .unwrap();
        let err = compile_file(dir.join("main.vrn")).unwrap_err().to_string();
        assert!(err.contains("module namespace, not an item"), "{err}");
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn slug_builtin_is_typed() {
        let src = r#"
#[page] fn article(ctx: PageContext, title: String) -> Result<Html, PageError> {
    let canonical = slug(title);
    return Ok(html {<p>{{ canonical }}</p>});
}
route article GET "/" query title<String> public => article;
"#;
        let p = compile_source(src).unwrap();
        assert!(p.page("article").is_some());
    }
}
