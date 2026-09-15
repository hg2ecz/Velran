use crate::diagnostics::CompileError;
use crate::handler_types::StaticType;
use crate::type_semantics::represented_as;
use language_core::{BuiltinFunction, Expr, Program, SAFE_HTML_DOMAIN_ID, ValueType};
use std::collections::HashMap;

pub(super) fn handles(function: BuiltinFunction) -> bool {
    matches!(
        function,
        BuiltinFunction::SafeHtmlEmpty
            | BuiltinFunction::SafeHtmlText
            | BuiltinFunction::SafeHtmlElement
            | BuiltinFunction::SafeHtmlLink
            | BuiltinFunction::SafeHtmlConcat
    )
}

pub(super) fn infer(
    function: BuiltinFunction,
    args: &[Expr],
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<ValueType, CompileError> {
    let safe = ValueType::Domain(SAFE_HTML_DOMAIN_ID);
    let require = |index: usize, expected: ValueType| -> Result<(), CompileError> {
        let actual = crate::expression::infer_expr_type(&args[index], known, program)?;
        if actual == expected || represented_as(program, actual, expected) {
            Ok(())
        } else {
            Err(CompileError::Syntax(format!(
                "{}(...) argument {} has incompatible type",
                function.source_name(),
                index + 1
            )))
        }
    };
    match function {
        BuiltinFunction::SafeHtmlEmpty => {}
        BuiltinFunction::SafeHtmlText => require(0, ValueType::String)?,
        BuiltinFunction::SafeHtmlElement => {
            require(0, ValueType::String)?;
            require(1, safe)?;
        }
        BuiltinFunction::SafeHtmlLink => {
            require(0, ValueType::String)?;
            require(1, safe)?;
        }
        BuiltinFunction::SafeHtmlConcat => {
            require(0, safe)?;
            require(1, safe)?;
        }
        _ => unreachable!(),
    }
    Ok(safe)
}
