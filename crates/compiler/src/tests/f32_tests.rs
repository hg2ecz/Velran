use super::*;

#[test]
fn parses_f32_literals_and_arithmetic() {
    let program = Program::default();
    let expr = parse_expr("1.5f32 + 2.25f32", &program).expect("F32 expression");
    assert_eq!(
        infer_expr_type(&expr, &HashMap::new(), &program).unwrap(),
        ValueType::F32
    );
    assert!(matches!(expr, Expr::Binary { .. }));
}

#[test]
fn parses_negative_f32_literal() {
    let expr = parse_expr("-0.5f32", &Program::default()).expect("negative F32");
    match expr {
        Expr::F32(v) => assert_eq!(v.get(), -0.5),
        other => panic!("unexpected expression: {other:?}"),
    }
}

#[test]
fn rejects_unsuffixed_float_and_mixed_numeric_types() {
    let program = Program::default();
    assert!(parse_expr("1.5", &program).is_err());
    let mixed = parse_expr("1 + 0.5f32", &program).expect("lexically valid mixed expression");
    assert!(infer_expr_type(&mixed, &HashMap::new(), &program).is_err());
}
