use super::*;

#[test]
fn parses_f32_math_builtins() {
    let p = Program::default();
    for source in [
        "0.5f32.sin()",
        "0.5f32.cos()",
        "4.0f32.sqrt()",
        "(-1.5f32).abs()",
    ] {
        let expr = parse_expr(source, &p).unwrap();
        assert_eq!(
            infer_expr_type(&expr, &HashMap::new(), &p).unwrap(),
            ValueType::F32
        );
    }
}

#[test]
fn abs_accepts_int_and_timer_returns_int() {
    let p = Program::default();
    let abs = parse_expr("(-42).abs()", &p).unwrap();
    assert_eq!(
        infer_expr_type(&abs, &HashMap::new(), &p).unwrap(),
        ValueType::Int
    );
    let timer = parse_expr("std::time::Instant::now()", &p).unwrap();
    assert_eq!(
        infer_expr_type(&timer, &HashMap::new(), &p).unwrap(),
        ValueType::Int
    );
    let elapsed = parse_expr("started.elapsed().as_nanos()", &p).unwrap();
    let mut env = HashMap::new();
    env.insert("started".into(), StaticType::trusted_scalar(ValueType::Int));
    assert_eq!(infer_expr_type(&elapsed, &env, &p).unwrap(), ValueType::Int);
}

#[test]
fn rejects_wrong_math_builtin_types_and_arity() {
    let p = Program::default();
    let sin_int = parse_expr("1.sin()", &p).unwrap();
    assert!(infer_expr_type(&sin_int, &HashMap::new(), &p).is_err());
    let timer_arg = parse_expr("std::time::Instant::now(1)", &p);
    assert!(timer_arg.is_err());
    let abs_string = parse_expr("\"x\".abs()", &p).unwrap();
    assert!(infer_expr_type(&abs_string, &HashMap::new(), &p).is_err());
}

#[test]
fn public_cache_rejects_monotonic_timer_output() {
    let src = r#"
#[page] fn timed(ctx: PageContext) -> Result<Html, PageError> {
    let started = std::time::Instant::now();
    return Ok(html {<p>{{ started }}</p>});
}
route timed GET "/timed" public cache public ttl 60 => timed;
"#;
    assert!(compile_source(src).is_err());
}

#[test]
fn legacy_duplicate_math_and_timer_surface_is_rejected() {
    let p = Program::default();
    for source in [
        "sin(0.5f32)",
        "cos(0.5f32)",
        "sqrt(4.0f32)",
        "abs(-1.0f32)",
        "ln(2.0f32)",
        "log10(10.0f32)",
        "log(8.0f32, 2.0f32)",
        "exp(1.0f32)",
        "pow(2.0f32, 3.0f32)",
        "round(1.5f32)",
        "floor(1.5f32)",
        "ceil(1.5f32)",
        "monotonicNanos()",
    ] {
        let err = parse_expr(source, &p).unwrap_err();
        assert!(err.to_string().contains("legacy"), "{source}: {err}");
    }
}

#[test]
fn rust_like_string_methods_are_typed() {
    let p = Program::default();
    let mut env = HashMap::new();
    env.insert("text".into(), StaticType::trusted_scalar(ValueType::String));
    env.insert(
        "map".into(),
        StaticType::trusted_scalar(ValueType::StringDict),
    );

    for source in [
        "text.trim()",
        "text.trim_start()",
        "text.trim_end()",
        "text.to_lowercase()",
        "text.to_uppercase()",
        "text.replace(\"a\", \"b\")",
        "text.repeat(2)",
    ] {
        let expr = parse_expr(source, &p).unwrap();
        assert_eq!(
            infer_expr_type(&expr, &env, &p).unwrap(),
            ValueType::String,
            "{source}"
        );
    }
    for source in [
        "text.contains(\"ell\")",
        "text.starts_with(\"he\")",
        "text.ends_with(\"lo\")",
        "map.contains_key(\"name\")",
    ] {
        let expr = parse_expr(source, &p).unwrap();
        assert_eq!(
            infer_expr_type(&expr, &env, &p).unwrap(),
            ValueType::Bool,
            "{source}"
        );
    }
    let expr = parse_expr("text.chars().count()", &p).unwrap();
    assert_eq!(infer_expr_type(&expr, &env, &p).unwrap(), ValueType::Int);
    let expr = parse_expr("BTreeMap::new()", &p).unwrap();
    assert_eq!(
        infer_expr_type(&expr, &env, &p).unwrap(),
        ValueType::StringDict
    );

    let expr = parse_expr("\"vrn\".repeat(3)", &p).unwrap();
    assert_eq!(infer_expr_type(&expr, &env, &p).unwrap(), ValueType::String);
    let expr = parse_expr("text.to_lowercase().replace(\" \", \"-\")", &p).unwrap();
    assert_eq!(infer_expr_type(&expr, &env, &p).unwrap(), ValueType::String);
}

#[test]
fn legacy_duplicate_string_surface_is_rejected() {
    let p = Program::default();
    for source in [
        "trim(text)",
        "trimStart(text)",
        "trimEnd(text)",
        "lower(text)",
        "upper(text)",
        "contains(text, \"x\")",
        "startsWith(text, \"x\")",
        "endsWith(text, \"x\")",
        "replace(text, \"a\", \"b\")",
        "repeat(text, 2)",
        "stringLen(text)",
        "dict()",
        "containsKey(map, \"x\")",
    ] {
        let err = parse_expr(source, &p).unwrap_err();
        assert!(err.to_string().contains("legacy"), "{source}: {err}");
    }
}

#[test]
fn regex_builtins_parse_and_infer_types() {
    let p = Program::default();
    let env = HashMap::new();

    let matched = parse_expr("regexMatch(\"abc-12\", \"^[a-z]+-[0-9]+$\")", &p).unwrap();
    assert_eq!(
        infer_expr_type(&matched, &env, &p).unwrap(),
        ValueType::Bool
    );

    let replaced = parse_expr("regexReplace(\"a1\", \"[0-9]\", \"#\")", &p).unwrap();
    assert_eq!(
        infer_expr_type(&replaced, &env, &p).unwrap(),
        ValueType::String
    );

    let captures = parse_expr("regexCaptures(\"a1\", \"([a-z])([0-9])\")", &p).unwrap();
    assert_eq!(
        infer_expr_type(&captures, &env, &p).unwrap(),
        ValueType::StringDict
    );
}

#[test]
fn regex_builtins_reject_wrong_types_and_arity() {
    let p = Program::default();
    let env = HashMap::new();
    for source in [
        "regexMatch(1, \"x\")",
        "regexReplace(\"x\", \"x\")",
        "regexCaptures(\"x\", 1)",
    ] {
        let expr = parse_expr(source, &p).unwrap();
        assert!(infer_expr_type(&expr, &env, &p).is_err(), "{source}");
    }
}

#[test]
fn parses_numeric_methods_on_variables_inside_arithmetic() {
    let p = Program::default();
    let expr = parse_expr("a64.sin() + 0.5f32 * a256.sin()", &p)
        .expect("Rust-like numeric methods on variables should parse");

    let mut locals = HashMap::new();
    locals.insert(
        "a64".into(),
        crate::handler_types::StaticType::trusted_scalar(ValueType::F32),
    );
    locals.insert(
        "a256".into(),
        crate::handler_types::StaticType::trusted_scalar(ValueType::F32),
    );
    assert_eq!(infer_expr_type(&expr, &locals, &p).unwrap(), ValueType::F32);
}
