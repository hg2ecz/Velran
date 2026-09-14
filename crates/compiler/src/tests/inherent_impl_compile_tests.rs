use super::*;

const METHOD_APP: &str = r#"
struct Summary {
    length: i64,
}

fn makeSummary() -> Summary {
    return Summary { length: 7 };
}

impl Summary {
    fn length_value(&self) -> i64 {
        return self.length;
    }
}

#[page]
fn home(ctx: PageContext) -> Result<Html, PageError> {
    let summary = makeSummary();
    let length = summary.length_value();
    return Ok(html {<p>{{ length }}</p>});
}
route home GET "/" public => home;
"#;

#[test]
fn inherent_self_method_compiles_through_verified_pure_path() {
    let program = compile_source(METHOD_APP).expect("inherent &self method should compile");
    let method = program
        .inherent_method("Summary", "length_value")
        .expect("registered method");
    assert!(method.has_receiver);
    let helper = program
        .pure_function(&method.function)
        .expect("method lowered to pure helper");
    assert_eq!(helper.params.len(), 1);
    assert!(matches!(
        helper.params[0].ty,
        language_core::PureParamType::Struct(_)
    ));
}

#[test]
fn mutable_receiver_is_rejected_fail_closed() {
    let bad = METHOD_APP.replace("&self", "&mut self");
    let err = compile_source(&bad).unwrap_err();
    assert!(err.to_string().contains("immutable `&self`"));
}

#[test]
fn unsafe_inside_impl_is_rejected_before_lowering() {
    let bad = METHOD_APP.replace("return self.length;", "unsafe { return self.length; }");
    let err = compile_source(&bad).unwrap_err();
    assert!(err.to_string().contains("SEC-RUST-001"));
}

#[test]
fn trait_impl_is_rejected_until_trait_iteration() {
    let bad = METHOD_APP.replace("impl Summary {", "impl Display for Summary {");
    let err = compile_source(&bad).unwrap_err();
    assert!(
        err.to_string()
            .contains("trait impls are not supported yet")
    );
}

#[test]
fn associated_constructor_with_self_compiles_through_verified_pure_path() {
    let source = r#"
pub struct Summary {
    pub length: i64,
}

impl Summary {
    pub fn new(length: &str) -> Self {
        return Self { length: length.chars().count() };
    }

    pub fn length_value(&self) -> i64 {
        return self.length;
    }
}

#[page]
fn home(ctx: PageContext) -> Result<Html, PageError> {
    let text = "hello";
    let summary = Summary::new(&text);
    let length = summary.length_value();
    return Ok(html {<p>{{ length }}</p>});
}
route home GET "/" public => home;
"#;
    let program = compile_source(source).expect("associated constructor using Self should compile");
    let constructor = program
        .inherent_method("Summary", "new")
        .expect("registered constructor");
    assert!(!constructor.has_receiver);
    let helper = program
        .pure_function(&constructor.function)
        .expect("constructor lowered to pure helper");
    assert!(matches!(
        helper.return_type,
        language_core::PureReturnType::Value(language_core::PureValueType::Struct(_))
    ));
}

#[test]
fn public_method_cannot_expose_private_struct_in_return_type() {
    let source = r#"
struct Secret { value: i64 }
pub struct Public { pub value: i64 }
impl Public {
    pub fn leak(&self) -> Secret { return Secret { value: self.value }; }
}
#[page]
fn home(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {<p>ok</p>}); }
route home GET "/" public => home;
"#;
    let err = compile_source(source).unwrap_err();
    assert!(err.to_string().contains("returns private struct"), "{err}");
}
