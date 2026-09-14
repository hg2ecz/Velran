use super::*;

fn unit(path: &str, module_path: &[&str], source: &str) -> crate::source_loader::SourceUnit {
    crate::source_loader::SourceUnit {
        path: PathBuf::from(path),
        module_path: module_path
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        namespace: module_path.join("::"),
        raw_sha256: String::new(),
        raw_len: source.len() as u64,
        raw_modified: None,
        source: crate::rustlike_surface::normalize(source).unwrap(),
        module_declarations: Vec::new(),
        public_reexports: Vec::new(),
        package_root: Vec::new(),
    }
}

const ROOT: &str = r#"
#[page]
fn home(ctx: PageContext) -> Result<Html, PageError> {
    let summary = metrics::build_summary();
    let length = summary.length_value();
    return Ok(html {<p>{{ length }}</p>});
}
route home GET "/" public => home;
"#;

const PUBLIC_METRICS: &str = r#"
pub struct Summary {
    pub length: i64,
    hidden: i64,
}

pub fn build_summary() -> Summary {
    return Summary { length: 7, hidden: 9 };
}

impl Summary {
    pub fn length_value(&self) -> i64 {
        return self.length;
    }

    fn hidden_value(&self) -> i64 {
        return self.hidden;
    }
}
"#;

#[test]
fn public_library_surface_is_accessible_cross_module() {
    let units = vec![
        unit("main.vrn", &[], ROOT),
        unit("metrics.vrn", &["metrics"], PUBLIC_METRICS),
    ];
    crate::compile_units(&units).expect("public library API should compile");
}

#[test]
fn private_function_is_rejected_cross_module() {
    let metrics = PUBLIC_METRICS.replace("pub fn build_summary", "fn build_summary");
    let units = vec![
        unit("main.vrn", &[], ROOT),
        unit("metrics.vrn", &["metrics"], &metrics),
    ];
    let err = crate::compile_units(&units).unwrap_err();
    assert!(err.to_string().contains("private function"));
}

#[test]
fn private_method_is_rejected_cross_module() {
    let root = ROOT.replace("length_value()", "hidden_value()");
    let units = vec![
        unit("main.vrn", &[], &root),
        unit("metrics.vrn", &["metrics"], PUBLIC_METRICS),
    ];
    let err = crate::compile_units(&units).unwrap_err();
    assert!(err.to_string().contains("private method"));
}

#[test]
fn private_field_is_rejected_cross_module() {
    let root = ROOT.replace(
        "let length = summary.length_value();",
        "let length = summary.hidden;",
    );
    let units = vec![
        unit("main.vrn", &[], &root),
        unit("metrics.vrn", &["metrics"], PUBLIC_METRICS),
    ];
    let err = crate::compile_units(&units).unwrap_err();
    assert!(err.to_string().contains("private field"));
}
