use super::*;
use std::time::{SystemTime, UNIX_EPOCH};
struct Ws {
    root: PathBuf,
}
impl Ws {
    fn new() -> Self {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("velran-pkg-{}-{n}", std::process::id()));
        fs::create_dir_all(root.join("app")).unwrap();
        fs::create_dir_all(root.join("libs/textkit")).unwrap();
        Self { root }
    }
    fn write(&self, p: &str, s: &str) {
        let p = self.root.join(p);
        if let Some(d) = p.parent() {
            fs::create_dir_all(d).unwrap()
        }
        fs::write(p, s).unwrap()
    }
    fn entry(&self) -> PathBuf {
        self.root.join("app/main.vrn")
    }
}
impl Drop for Ws {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
const APP: &str = "[package]\nname=\"demo_app\"\nkind=\"app\"\nentry=\"main.vrn\"\n[dependencies]\ntextkit={path=\"../libs/textkit\"}\n";
const LIB: &str = "[package]\nname=\"textkit\"\nkind=\"lib\"\nentry=\"lib.vrn\"\n";
fn app(call: &str) -> String {
    format!(
        "#[page]\nfn home(ctx: PageContext) -> Result<Html, PageError> {{ let value = {call}; return Ok(html {{<p>{{{{ value }}}}</p>}}); }}\nroute home GET \"/\" public => home;\n"
    )
}
#[test]
fn public_local_library_api_compiles() {
    let w = Ws::new();
    w.write("app/velran.toml", APP);
    w.write("app/main.vrn", &app("textkit::answer()"));
    w.write("libs/textkit/velran.toml", LIB);
    w.write(
        "libs/textkit/lib.vrn",
        "pub fn answer() -> i64 { return 42; }\n",
    );
    let c = compile_file_with_dependencies(w.entry()).unwrap();
    assert!(c.source_roots.iter().any(|p| p.ends_with("textkit")));
    assert!(c.source_files.iter().any(|p| p.ends_with("velran.toml")));
}
#[test]
fn private_library_api_is_rejected() {
    let w = Ws::new();
    w.write("app/velran.toml", APP);
    w.write("app/main.vrn", &app("textkit::answer()"));
    w.write("libs/textkit/velran.toml", LIB);
    w.write(
        "libs/textkit/lib.vrn",
        "fn answer() -> i64 { return 42; }\n",
    );
    assert!(
        compile_file(w.entry())
            .unwrap_err()
            .to_string()
            .contains("private")
    );
}
#[test]
fn library_web_authority_is_rejected() {
    let w = Ws::new();
    w.write("app/velran.toml", APP);
    w.write("app/main.vrn", &app("textkit::answer()"));
    w.write("libs/textkit/velran.toml", LIB);
    w.write(
        "libs/textkit/lib.vrn",
        "pub fn answer() -> i64 { return 42; }\nroute hidden GET \"/hidden\" public => hidden;\n",
    );
    assert!(
        compile_file(w.entry())
            .unwrap_err()
            .to_string()
            .contains("SEC-PKG-001")
    );
}

#[test]
fn transitive_local_library_dependency_compiles_without_leaking_namespace() {
    let w = Ws::new();
    w.write("app/velran.toml", APP);
    w.write("app/main.vrn", &app("textkit::answer()"));
    w.write(
        "libs/textkit/velran.toml",
        "[package]\nname=\"textkit\"\nkind=\"lib\"\nentry=\"lib.vrn\"\n[dependencies]\nutil={path=\"../util\"}\n",
    );
    w.write(
        "libs/textkit/lib.vrn",
        "use util as util;\npub fn answer() -> i64 { return util::answer(); }\n",
    );
    w.write(
        "libs/util/velran.toml",
        "[package]\nname=\"util\"\nkind=\"lib\"\nentry=\"lib.vrn\"\n",
    );
    w.write(
        "libs/util/lib.vrn",
        "pub fn answer() -> i64 { return 42; }\n",
    );

    let compiled = compile_file_with_dependencies(w.entry()).unwrap();
    assert!(compiled.source_roots.iter().any(|p| p.ends_with("util")));
    assert!(
        compiled
            .source_files
            .iter()
            .filter(|p| p.ends_with("velran.toml"))
            .count()
            >= 3
    );
}

#[test]
fn transitive_dependency_is_private_to_declaring_library() {
    let w = Ws::new();
    w.write("app/velran.toml", APP);
    w.write("app/main.vrn", &app("textkit::util::answer()"));
    w.write(
        "libs/textkit/velran.toml",
        "[package]\nname=\"textkit\"\nkind=\"lib\"\nentry=\"lib.vrn\"\n[dependencies]\nutil={path=\"../util\"}\n",
    );
    w.write(
        "libs/textkit/lib.vrn",
        "pub fn answer() -> i64 { return 1; }\n",
    );
    w.write(
        "libs/util/velran.toml",
        "[package]\nname=\"util\"\nkind=\"lib\"\nentry=\"lib.vrn\"\n",
    );
    w.write(
        "libs/util/lib.vrn",
        "pub fn answer() -> i64 { return 42; }\n",
    );

    assert!(
        compile_file(w.entry())
            .unwrap_err()
            .to_string()
            .contains("private")
    );
}

#[test]
fn transitive_dependency_cycle_is_rejected() {
    let w = Ws::new();
    w.write("app/velran.toml", APP);
    w.write("app/main.vrn", &app("textkit::answer()"));
    w.write(
        "libs/textkit/velran.toml",
        "[package]\nname=\"textkit\"\nkind=\"lib\"\nentry=\"lib.vrn\"\n[dependencies]\nutil={path=\"../util\"}\n",
    );
    w.write(
        "libs/textkit/lib.vrn",
        "pub fn answer() -> i64 { return 1; }\n",
    );
    w.write(
        "libs/util/velran.toml",
        "[package]\nname=\"util\"\nkind=\"lib\"\nentry=\"lib.vrn\"\n[dependencies]\ntextkit={path=\"../textkit\"}\n",
    );
    w.write(
        "libs/util/lib.vrn",
        "pub fn answer() -> i64 { return 42; }\n",
    );

    let error = compile_file(w.entry()).unwrap_err().to_string();
    assert!(error.contains("dependency cycle"), "{error}");
    assert!(error.contains("textkit -> util -> textkit"), "{error}");
}

#[test]
fn same_package_name_cannot_resolve_to_multiple_roots() {
    let w = Ws::new();
    w.write(
        "app/velran.toml",
        "[package]\nname=\"demo_app\"\nkind=\"app\"\nentry=\"main.vrn\"\n[dependencies]\na={path=\"../libs/a\"}\nb={path=\"../libs/b\"}\n",
    );
    w.write("app/main.vrn", &app("a::answer()"));
    w.write(
        "libs/a/velran.toml",
        "[package]\nname=\"a\"\nkind=\"lib\"\nentry=\"lib.vrn\"\n[dependencies]\nshared={path=\"../shared-one\"}\n",
    );
    w.write("libs/a/lib.vrn", "pub fn answer() -> i64 { return 1; }\n");
    w.write(
        "libs/b/velran.toml",
        "[package]\nname=\"b\"\nkind=\"lib\"\nentry=\"lib.vrn\"\n[dependencies]\nshared={path=\"../shared-two\"}\n",
    );
    w.write("libs/b/lib.vrn", "pub fn answer() -> i64 { return 2; }\n");
    for root in ["shared-one", "shared-two"] {
        w.write(
            &format!("libs/{root}/velran.toml"),
            "[package]\nname=\"shared\"\nkind=\"lib\"\nentry=\"lib.vrn\"\n",
        );
        w.write(
            &format!("libs/{root}/lib.vrn"),
            "pub fn answer() -> i64 { return 3; }\n",
        );
    }

    let error = compile_file(w.entry()).unwrap_err().to_string();
    assert!(
        error.contains("resolves to multiple package roots"),
        "{error}"
    );
}

#[test]
fn cross_package_struct_constructor_and_method_compile() {
    let w = Ws::new();
    w.write("app/velran.toml", APP);
    w.write(
        "app/main.vrn",
        r#"#[page]
fn home(ctx: PageContext) -> Result<Html, PageError> {
    let text = "hello";
    let summary = textkit::Summary::new(&text);
    let value = summary.length_value();
    return Ok(html {<p>{{ value }}</p>});
}
route home GET "/" public => home;
"#,
    );
    w.write("libs/textkit/velran.toml", LIB);
    w.write(
        "libs/textkit/lib.vrn",
        r#"pub struct Summary {
    pub length: i64,
    hidden: i64,
}
impl Summary {
    pub fn new(text: &str) -> Self {
        return Self { length: text.chars().count(), hidden: 7 };
    }
    pub fn length_value(&self) -> i64 { return self.length; }
}
"#,
    );
    compile_file_with_dependencies(w.entry()).expect("cross-package object API should compile");
}

#[test]
fn cross_package_private_struct_field_is_rejected() {
    let w = Ws::new();
    w.write("app/velran.toml", APP);
    w.write(
        "app/main.vrn",
        r#"#[page]
fn home(ctx: PageContext) -> Result<Html, PageError> {
    let text = "hello";
    let summary = textkit::Summary::new(&text);
    let value = summary.hidden;
    return Ok(html {<p>{{ value }}</p>});
}
route home GET "/" public => home;
"#,
    );
    w.write("libs/textkit/velran.toml", LIB);
    w.write(
        "libs/textkit/lib.vrn",
        r#"pub struct Summary { pub length: i64, hidden: i64 }
impl Summary {
    pub fn new(text: &str) -> Self {
        return Self { length: text.chars().count(), hidden: 7 };
    }
}
"#,
    );
    let err = compile_file(w.entry()).unwrap_err().to_string();
    assert!(err.contains("private field"), "{err}");
}

#[test]
fn package_root_can_reexport_public_child_module_namespace() {
    let w = Ws::new();
    w.write("app/velran.toml", APP);
    w.write("app/main.vrn", &app("textkit::api::answer()"));
    w.write("libs/textkit/velran.toml", LIB);
    w.write(
        "libs/textkit/lib.vrn",
        "pub mod core;\npub use crate::core as api;\n",
    );
    w.write(
        "libs/textkit/core.vrn",
        "pub fn answer() -> i64 { return 42; }\n",
    );
    compile_file_with_dependencies(w.entry())
        .expect("public module namespace re-export should compile");
}

#[test]
fn package_root_cannot_reexport_private_child_module() {
    let w = Ws::new();
    w.write("app/velran.toml", APP);
    w.write("app/main.vrn", &app("textkit::api::answer()"));
    w.write("libs/textkit/velran.toml", LIB);
    w.write(
        "libs/textkit/lib.vrn",
        "mod core;\npub use crate::core as api;\n",
    );
    w.write(
        "libs/textkit/core.vrn",
        "pub fn answer() -> i64 { return 42; }\n",
    );
    let err = compile_file(w.entry()).unwrap_err().to_string();
    assert!(err.contains("crosses private module"), "{err}");
}

#[test]
fn package_cannot_reexport_private_transitive_dependency() {
    let w = Ws::new();
    w.write("app/velran.toml", APP);
    w.write("app/main.vrn", &app("textkit::util_api::answer()"));
    w.write(
        "libs/textkit/velran.toml",
        "[package]\nname=\"textkit\"\nkind=\"lib\"\nentry=\"lib.vrn\"\n[dependencies]\nutil={path=\"../util\"}\n",
    );
    w.write("libs/textkit/lib.vrn", "pub use util as util_api;\n");
    w.write(
        "libs/util/velran.toml",
        "[package]\nname=\"util\"\nkind=\"lib\"\nentry=\"lib.vrn\"\n",
    );
    w.write(
        "libs/util/lib.vrn",
        "pub fn answer() -> i64 { return 42; }\n",
    );
    let err = compile_file(w.entry()).unwrap_err().to_string();
    assert!(err.contains("crosses private module"), "{err}");
}
