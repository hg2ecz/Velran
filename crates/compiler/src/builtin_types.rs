use crate::credential_builtin_types;
use crate::diagnostics::CompileError;
use crate::handler_types::StaticType;
use crate::math_builtin_types;
use crate::regex_types;
use crate::string_builtin_types;
use crate::type_semantics::represented_as;
use language_core::{BuiltinFunction, Expr, Program, ValueType};
use std::collections::HashMap;

pub(super) fn infer_builtin_type(
    function: BuiltinFunction,
    args: &[Expr],
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<ValueType, CompileError> {
    if !function.accepts_arity(args.len()) {
        let metadata = function.metadata();
        let expected = if metadata.min_args == metadata.max_args {
            metadata.min_args.to_string()
        } else {
            format!("{}..={}", metadata.min_args, metadata.max_args)
        };
        return Err(CompileError::Syntax(format!(
            "{}(...) expects {expected} argument(s), got {}",
            metadata.source_name,
            args.len()
        )));
    }

    if math_builtin_types::handles(function) {
        return math_builtin_types::infer(function, args, known, program);
    }
    if string_builtin_types::handles(function) {
        return string_builtin_types::infer(function, args, known, program);
    }
    if credential_builtin_types::handles(function) {
        return credential_builtin_types::infer(function, args, known, program);
    }

    match function {
        BuiltinFunction::DictNew => Ok(ValueType::StringDict),
        BuiltinFunction::ContainsKey => {
            require_types(
                function,
                args,
                &[ValueType::StringDict, ValueType::String],
                known,
                program,
            )?;
            Ok(ValueType::Bool)
        }
        BuiltinFunction::RemoveKey => {
            require_types(
                function,
                args,
                &[ValueType::StringDict, ValueType::String],
                known,
                program,
            )?;
            Ok(ValueType::StringDict)
        }
        BuiltinFunction::RegexMatch
        | BuiltinFunction::RegexReplace
        | BuiltinFunction::RegexCaptures => {
            regex_types::infer_regex_builtin_type(function, args, known, program)
        }
        BuiltinFunction::Redact => {
            let ty = crate::expression::infer_expr_type(&args[0], known, program)?;
            if matches!(ty, ValueType::Upload | ValueType::Image) {
                return Err(CompileError::security(
                    "SEC-DATA-012",
                    "redact(...) requires a scalar data value",
                    Some("redact a scalar field or identifier representation, not an Upload/Image capability".into()),
                ));
            }
            Ok(ValueType::String)
        }
        _ => Err(CompileError::Syntax(format!(
            "internal: builtin `{}` has no type checker",
            function.source_name()
        ))),
    }
}

fn require_types(
    function: BuiltinFunction,
    args: &[Expr],
    expected: &[ValueType],
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<(), CompileError> {
    use crate::expression::infer_expr_type;

    if args.len() != expected.len() {
        return Err(signature_error(function, expected));
    }
    for (arg, expected_ty) in args.iter().zip(expected) {
        if !represented_as(program, infer_expr_type(arg, known, program)?, *expected_ty) {
            return Err(signature_error(function, expected));
        }
    }
    Ok(())
}

fn signature_error(function: BuiltinFunction, expected: &[ValueType]) -> CompileError {
    let expected = expected
        .iter()
        .map(|ty| format!("{ty:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    CompileError::Syntax(format!(
        "{}(...) requires ({expected})",
        function.source_name()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_builtins_are_strictly_typed() {
        let program = Program::default();
        let known = HashMap::new();
        assert_eq!(
            infer_builtin_type(BuiltinFunction::DictNew, &[], &known, &program).unwrap(),
            ValueType::StringDict
        );
    }
}
