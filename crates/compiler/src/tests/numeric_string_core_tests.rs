use super::*;

#[test]
fn arithmetic_bitwise_and_logical_operators_are_typed() {
    let program = Program::default();
    let known = HashMap::new();

    for source in [
        "17 % 5", "3 << 2", "12 >> 1", "12 & 10", "12 ^ 10", "12 | 10",
    ] {
        let expr = parse_expr(source, &program).unwrap();
        assert_eq!(
            infer_expr_type(&expr, &known, &program).unwrap(),
            ValueType::Int,
            "{source}"
        );
    }

    for source in ["true && false", "true || false", "!false", "1 < 2 && 3 < 4"] {
        let expr = parse_expr(source, &program).unwrap();
        assert_eq!(
            infer_expr_type(&expr, &known, &program).unwrap(),
            ValueType::Bool,
            "{source}"
        );
    }
}

#[test]
fn operator_precedence_keeps_arithmetic_shift_comparison_and_logic_layers() {
    let program = Program::default();
    let expr = parse_expr("1 + 2 * 3 << 1 == 14 && true", &program).unwrap();
    let Expr::Binary {
        op: BinaryOp::LogicalAnd,
        left,
        ..
    } = expr
    else {
        panic!("logical and expected at root");
    };
    assert!(matches!(
        *left,
        Expr::Binary {
            op: BinaryOp::Eq,
            ..
        }
    ));
}

#[test]
fn extended_math_builtins_are_strictly_f32() {
    let program = Program::default();
    let known = HashMap::new();

    for source in [
        "2.0f32.ln()",
        "100.0f32.log10()",
        "8.0f32.log(2.0f32)",
        "1.0f32.exp()",
        "2.0f32.powf(8.0f32)",
        "2.5f32.round()",
        "2.5f32.floor()",
        "2.5f32.ceil()",
    ] {
        let expr = parse_expr(source, &program).unwrap();
        assert_eq!(
            infer_expr_type(&expr, &known, &program).unwrap(),
            ValueType::F32,
            "{source}"
        );
    }

    let bad = parse_expr("2.powf(8)", &program).unwrap();
    assert!(infer_expr_type(&bad, &known, &program).is_err());
}

#[test]
fn extended_string_builtins_have_explicit_types() {
    let program = Program::default();
    let known = HashMap::new();

    for source in [
        "\" x \".trim_start()",
        "\" x \".trim_end()",
        "\"ab\".repeat(3)",
    ] {
        let expr = parse_expr(source, &program).unwrap();
        assert_eq!(
            infer_expr_type(&expr, &known, &program).unwrap(),
            ValueType::String,
            "{source}"
        );
    }
}

#[test]
fn invalid_operator_types_are_rejected_at_compile_time() {
    let program = Program::default();
    let known = HashMap::new();

    for source in [
        "1 && 2",
        "true << 1",
        "1.0f32 & 2.0f32",
        "\"x\" % \"y\"",
        "!1",
    ] {
        let expr = parse_expr(source, &program).unwrap();
        assert!(
            infer_expr_type(&expr, &known, &program).is_err(),
            "{source}"
        );
    }
}
