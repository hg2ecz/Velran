use crate::diagnostics::CompileError;
use language_core::{Program, Visibility};

pub(crate) fn declaration_visibility(source: &str, keyword_pos: usize) -> Visibility {
    let line_start = source[..keyword_pos]
        .rfind('\n')
        .map(|value| value + 1)
        .unwrap_or(0);
    if source[line_start..keyword_pos].trim() == "pub" {
        Visibility::Public
    } else {
        Visibility::Private
    }
}

pub(crate) fn strip_member_visibility(raw: &str) -> (Visibility, &str) {
    let trimmed = raw.trim();
    match trimmed.strip_prefix("pub ") {
        Some(rest) => (Visibility::Public, rest.trim_start()),
        None => (Visibility::Private, trimmed),
    }
}

pub(crate) fn register(program: &mut Program, symbol: impl Into<String>, visibility: Visibility) {
    let symbol = symbol.into();
    let owner_namespace = symbol
        .rsplit_once("::")
        .map(|(owner, _)| owner)
        .unwrap_or("")
        .to_string();
    program.set_visibility(symbol, owner_namespace, visibility);
}

pub(crate) fn register_member(
    program: &mut Program,
    parent_symbol: &str,
    member: &str,
    visibility: Visibility,
) {
    let owner_namespace = parent_symbol
        .rsplit_once("::")
        .map(|(owner, _)| owner)
        .unwrap_or("");
    program.set_visibility(
        format!("{parent_symbol}::{member}"),
        owner_namespace,
        visibility,
    );
}

pub(crate) fn require_access(
    program: &Program,
    symbol: &str,
    namespace: &str,
    kind: &str,
) -> Result<(), CompileError> {
    let owner_module = program
        .visibilities
        .iter()
        .find(|entry| entry.symbol == symbol)
        .map(|entry| entry.owner_namespace.as_str())
        .unwrap_or("");
    if !program.module_path_is_accessible_from(owner_module, namespace) {
        return Err(CompileError::Syntax(format!(
            "private module path `{owner_module}` is not accessible from module `{}`; expose the required module with `pub mod`",
            if namespace.is_empty() {
                "<root>"
            } else {
                namespace
            }
        )));
    }
    if program.symbol_is_accessible_from(symbol, namespace) {
        Ok(())
    } else {
        Err(CompileError::Syntax(format!(
            "private {kind} `{symbol}` is not accessible from module `{}`; mark it `pub` to expose it",
            if namespace.is_empty() {
                "<root>"
            } else {
                namespace
            }
        )))
    }
}

pub(crate) fn validate_pure_expr_access(
    expr: &language_core::Expr,
    known: &std::collections::HashMap<String, crate::handler_types::StaticType>,
    program: &Program,
    namespace: &str,
) -> Result<(), CompileError> {
    use language_core::Expr;
    match expr {
        Expr::Field { base, field } => {
            if let Some(crate::handler_types::StaticType::PureStruct(schema)) = known.get(base) {
                require_access(program, &format!("{schema}::{field}"), namespace, "field")?;
            }
        }
        Expr::Slugify(inner) | Expr::Not(inner) => {
            validate_pure_expr_access(inner, known, program, namespace)?;
        }
        Expr::Builtin { args, .. } => {
            for arg in args {
                validate_pure_expr_access(arg, known, program, namespace)?;
            }
        }
        Expr::F32ArrayNew { len, fill } => {
            validate_pure_expr_access(len, known, program, namespace)?;
            validate_pure_expr_access(fill, known, program, namespace)?;
        }
        Expr::CollectionIndex { index, .. } => {
            validate_pure_expr_access(index, known, program, namespace)?;
        }
        Expr::PureSumUnwrapOr { fallback, .. } => {
            validate_pure_expr_access(fallback, known, program, namespace)?;
        }
        Expr::Binary { left, right, .. } => {
            validate_pure_expr_access(left, known, program, namespace)?;
            validate_pure_expr_access(right, known, program, namespace)?;
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn validate_public_pure_api(
    function_name: &str,
    params: &[language_core::PureFunctionParam],
    return_type: &language_core::PureReturnType,
    program: &Program,
) -> Result<(), CompileError> {
    for param in params {
        if let language_core::PureParamType::Struct(id) = param.ty {
            let schema = program.json_schema_by_id(id).ok_or_else(|| {
                CompileError::Syntax(format!(
                    "public function `{function_name}` references unknown struct parameter id {id}"
                ))
            })?;
            if program.visibility(&schema.name) != Visibility::Public {
                return Err(CompileError::Syntax(format!(
                    "public function `{function_name}` exposes private struct `{}` through parameter `{}`; mark the struct `pub` or keep the function private",
                    schema.name, param.name
                )));
            }
        }
    }
    fn validate_return_value(
        function_name: &str,
        value: &language_core::PureValueType,
        program: &Program,
    ) -> Result<(), CompileError> {
        if let language_core::PureValueType::Struct(name) = value {
            if program.visibility(name) != Visibility::Public {
                return Err(CompileError::Syntax(format!(
                    "public function `{function_name}` returns private struct `{name}`; mark the struct `pub` or keep the function private"
                )));
            }
        }
        Ok(())
    }

    match return_type {
        language_core::PureReturnType::Value(value)
        | language_core::PureReturnType::Option(value) => {
            validate_return_value(function_name, value, program)?;
        }
        language_core::PureReturnType::Result { ok, err } => {
            validate_return_value(function_name, ok, program)?;
            validate_return_value(function_name, err, program)?;
        }
        language_core::PureReturnType::Unit => {}
    }
    Ok(())
}
