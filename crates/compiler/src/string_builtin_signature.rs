use crate::diagnostics::CompileError;
use crate::expression::infer_expr_type;
use crate::handler_types::StaticType;
use crate::type_semantics::represented_as;
use language_core::{BuiltinFunction, Expr, Program, ValueType};
use std::collections::HashMap;

pub(super) fn require_split_bound(args: &[Expr]) -> Result<(), CompileError> {
    match args.get(2) {
        Some(Expr::Int(max)) if (1..=4096).contains(max) => Ok(()),
        _ => Err(CompileError::Syntax(
            "splitBounded(..., maxItems) requires an integer literal in 1..4096".into(),
        )),
    }
}

pub(super) fn require_types(
    function: BuiltinFunction,
    args: &[Expr],
    expected: &[ValueType],
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<(), CompileError> {
    if args.len() != expected.len() {
        return Err(signature_error(function, &format_types(expected)));
    }
    for (arg, expected_ty) in args.iter().zip(expected) {
        if !represented_as(program, infer_expr_type(arg, known, program)?, *expected_ty) {
            return Err(signature_error(function, &format_types(expected)));
        }
    }
    Ok(())
}

fn format_types(types: &[ValueType]) -> String {
    types
        .iter()
        .map(|ty| format!("{ty:?}"))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn signature_error(function: BuiltinFunction, expected: &str) -> CompileError {
    CompileError::Syntax(format!(
        "{}(...) requires ({expected})",
        function.source_name()
    ))
}
