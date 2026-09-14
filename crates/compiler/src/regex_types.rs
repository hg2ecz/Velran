use crate::diagnostics::CompileError;
use crate::expression::infer_expr_type;
use crate::handler_types::StaticType;
use language_core::{BuiltinFunction, Expr, Program, ValueType};
use std::collections::HashMap;

pub(super) fn infer_regex_builtin_type(
    function: BuiltinFunction,
    args: &[Expr],
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<ValueType, CompileError> {
    let require_strings = |expected: usize, signature: &str| -> Result<(), CompileError> {
        if args.len() != expected
            || args
                .iter()
                .any(|arg| infer_expr_type(arg, known, program).ok() != Some(ValueType::String))
        {
            return Err(CompileError::Syntax(signature.into()));
        }
        Ok(())
    };

    match function {
        BuiltinFunction::RegexMatch => {
            require_strings(
                2,
                "regexMatch(text, pattern) requires exactly two String arguments",
            )?;
            Ok(ValueType::Bool)
        }
        BuiltinFunction::RegexReplace => {
            require_strings(
                3,
                "regexReplace(text, pattern, replacement) requires exactly three String arguments",
            )?;
            Ok(ValueType::String)
        }
        BuiltinFunction::RegexCaptures => {
            require_strings(
                2,
                "regexCaptures(text, pattern) requires exactly two String arguments",
            )?;
            Ok(ValueType::StringDict)
        }
        _ => Err(CompileError::Syntax(
            "internal: non-regex builtin passed to regex type checker".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_builtins_are_strictly_typed() {
        let p = Program::default();
        let known = HashMap::new();
        assert_eq!(
            infer_regex_builtin_type(
                BuiltinFunction::RegexMatch,
                &[Expr::String("abc".into()), Expr::String("^a".into())],
                &known,
                &p,
            )
            .unwrap(),
            ValueType::Bool
        );
        assert_eq!(
            infer_regex_builtin_type(
                BuiltinFunction::RegexCaptures,
                &[Expr::String("abc".into()), Expr::String("(a)".into())],
                &known,
                &p,
            )
            .unwrap(),
            ValueType::StringDict
        );
    }
}
