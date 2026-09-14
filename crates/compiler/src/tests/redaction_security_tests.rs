use super::*;
use crate::scalar_security::ScalarType;
use language_core::{DataSensitivity, Expr, ValueType};

#[test]
fn redact_turns_classified_scalar_into_redacted_string() {
    let program = Program::default();
    let mut known = HashMap::new();
    known.insert(
        "secret".into(),
        StaticType::Scalar(ScalarType::classified(
            ValueType::String,
            DataSensitivity::Secret,
        )),
    );
    let expr = Expr::Builtin {
        function: BuiltinFunction::Redact,
        args: vec![Expr::Variable("secret".into())],
    };
    let ty = infer_static_expr_type(&expr, &known, &program).unwrap();
    let scalar = ty.scalar().unwrap();
    assert_eq!(scalar.value_type, ValueType::String);
    assert_eq!(scalar.sensitivity, DataSensitivity::Redacted);
}

#[test]
fn redacted_cannot_be_forged_with_a_type_annotation() {
    let program = Program::default();
    assert!(
        crate::type_resolution::resolve_annotated_value_type("Redacted<String>", "app", &program,)
            .is_none()
    );
}
