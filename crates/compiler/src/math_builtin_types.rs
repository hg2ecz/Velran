use crate::diagnostics::CompileError;
use crate::expression::infer_expr_type;
use crate::handler_types::StaticType;
use language_core::{BuiltinFunction, Expr, Program, ValueType};
use std::collections::HashMap;

pub(super) fn handles(function: BuiltinFunction) -> bool {
    matches!(
        function,
        BuiltinFunction::Sin
            | BuiltinFunction::Cos
            | BuiltinFunction::Sqrt
            | BuiltinFunction::Abs
            | BuiltinFunction::Ln
            | BuiltinFunction::Log10
            | BuiltinFunction::Log
            | BuiltinFunction::Exp
            | BuiltinFunction::Pow
            | BuiltinFunction::Round
            | BuiltinFunction::Floor
            | BuiltinFunction::Ceil
            | BuiltinFunction::MonotonicNanos
            | BuiltinFunction::ToF32
    )
}

pub(super) fn infer(
    function: BuiltinFunction,
    args: &[Expr],
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<ValueType, CompileError> {
    match function {
        BuiltinFunction::Sin
        | BuiltinFunction::Cos
        | BuiltinFunction::Sqrt
        | BuiltinFunction::Ln
        | BuiltinFunction::Log10
        | BuiltinFunction::Exp
        | BuiltinFunction::Round
        | BuiltinFunction::Floor
        | BuiltinFunction::Ceil => require_unary_f32(function, args, known, program),
        BuiltinFunction::Log | BuiltinFunction::Pow => {
            require_f32_args(function, args, 2, known, program)?;
            Ok(ValueType::F32)
        }
        BuiltinFunction::Abs => {
            require_arity(function, args, 1)?;
            let ty = infer_expr_type(&args[0], known, program)?;
            if matches!(ty, ValueType::Int | ValueType::F32) {
                Ok(ty)
            } else {
                Err(type_error(function, "Int or F32"))
            }
        }
        BuiltinFunction::MonotonicNanos => {
            require_arity(function, args, 0)?;
            Ok(ValueType::Int)
        }
        BuiltinFunction::ToF32 => {
            require_arity(function, args, 1)?;
            if infer_expr_type(&args[0], known, program)? == ValueType::Int {
                Ok(ValueType::F32)
            } else {
                Err(type_error(function, "Int"))
            }
        }
        _ => Err(CompileError::Syntax(
            "internal: non-math builtin routed to math type checker".into(),
        )),
    }
}

fn require_unary_f32(
    function: BuiltinFunction,
    args: &[Expr],
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<ValueType, CompileError> {
    require_f32_args(function, args, 1, known, program)?;
    Ok(ValueType::F32)
}

fn require_f32_args(
    function: BuiltinFunction,
    args: &[Expr],
    count: usize,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<(), CompileError> {
    require_arity(function, args, count)?;
    if args
        .iter()
        .all(|arg| infer_expr_type(arg, known, program).ok() == Some(ValueType::F32))
    {
        Ok(())
    } else {
        Err(type_error(function, &format!("{count} F32 argument(s)")))
    }
}

fn require_arity(
    function: BuiltinFunction,
    args: &[Expr],
    count: usize,
) -> Result<(), CompileError> {
    if args.len() == count {
        Ok(())
    } else {
        Err(CompileError::Syntax(format!(
            "{}(...) requires exactly {count} argument(s)",
            function.source_name()
        )))
    }
}

fn type_error(function: BuiltinFunction, expected: &str) -> CompileError {
    CompileError::Syntax(format!(
        "{}(...) requires {expected}",
        function.source_name()
    ))
}
