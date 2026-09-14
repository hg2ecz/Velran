use crate::diagnostics::CompileError;
use crate::expression::{infer_static_expr_type, parse_expr_in_namespace, validate_expr};
use crate::handler_types::StaticType;
use crate::module_namespace::resolve;
use crate::response_security::{ResponseBoundary, validate_response_expression};
use crate::source_syntax::{is_identifier, matching_brace};
use language_core::{Program, TypedJsonField, ValueType};
use std::collections::{BTreeMap, HashMap};

pub(crate) fn parse(
    raw: &str,
    namespace: &str,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<Option<(String, Vec<TypedJsonField>)>, CompileError> {
    let raw = raw.trim();
    let Some(inner) = raw.strip_prefix("Json(").and_then(|v| v.strip_suffix(')')) else {
        return Ok(None);
    };
    let inner = inner.trim();
    let open = inner.find('{').ok_or_else(|| {
        CompileError::Syntax(
            "typed JSON response expected `Json(Type { field: value, ... })`".into(),
        )
    })?;
    let close = matching_brace(inner, open).ok_or_else(|| {
        CompileError::Syntax("typed JSON response struct literal is unclosed".into())
    })?;
    if !inner[close + 1..].trim().is_empty() {
        return Err(CompileError::Syntax(
            "unexpected tokens after typed JSON response struct literal".into(),
        ));
    }
    let type_name = inner[..open].trim();
    if type_name.is_empty() || !type_name.split("::").all(is_identifier) {
        return Err(CompileError::Syntax(format!(
            "invalid typed JSON response type `{type_name}`"
        )));
    }
    let schema_name = if type_name.contains("::") {
        type_name.to_owned()
    } else {
        resolve(namespace, type_name)
    };
    let schema = program.json_schema(&schema_name).ok_or_else(|| {
        CompileError::Syntax(format!(
            "typed JSON response references unknown struct `{type_name}`"
        ))
    })?;

    let mut parsed = BTreeMap::<String, language_core::Expr>::new();
    for entry in split_fields(&inner[open + 1..close])? {
        let Some((name, expr_text)) = entry.split_once(':') else {
            return Err(CompileError::Syntax(format!(
                "typed JSON response field `{entry}` expected `name: expression`"
            )));
        };
        let name = name.trim();
        if !is_identifier(name) {
            return Err(CompileError::Syntax(format!(
                "invalid typed JSON response field `{name}`"
            )));
        }
        if parsed.contains_key(name) {
            return Err(CompileError::Syntax(format!(
                "duplicate typed JSON response field `{name}`"
            )));
        }
        let expr = parse_expr_in_namespace(expr_text.trim(), namespace, program)?;
        validate_expr(&expr, known, program)?;
        crate::visibility::validate_pure_expr_access(&expr, known, program, namespace)?;
        validate_response_expression(&expr, known, program, ResponseBoundary::Json)?;
        parsed.insert(name.to_owned(), expr);
    }

    if parsed.len() != schema.fields.len() {
        return Err(CompileError::Syntax(format!(
            "typed JSON response `{type_name}` must initialize exactly {} fields",
            schema.fields.len()
        )));
    }

    let mut fields = Vec::with_capacity(schema.fields.len());
    for field in &schema.fields {
        if !wire_supported(field.ty) {
            return Err(CompileError::Syntax(format!(
                "typed JSON response field `{}.{}` uses type not yet supported by the typed response ABI",
                type_name, field.name
            )));
        }
        let expr = parsed.remove(&field.name).ok_or_else(|| {
            CompileError::Syntax(format!(
                "typed JSON response `{type_name}` missing field `{}`",
                field.name
            ))
        })?;
        let actual = infer_static_expr_type(&expr, known, program)?;
        let matches = actual
            .scalar()
            .map(|v| v.value_type == field.ty)
            .unwrap_or(false);
        if !matches {
            return Err(CompileError::Syntax(format!(
                "typed JSON response field `{}.{}` has the wrong type",
                type_name, field.name
            )));
        }
        fields.push(TypedJsonField {
            name: field.name.clone(),
            ty: field.ty,
            expr,
        });
    }
    if let Some(extra) = parsed.keys().next() {
        return Err(CompileError::Syntax(format!(
            "typed JSON response `{type_name}` has unknown field `{extra}`"
        )));
    }
    Ok(Some((schema_name, fields)))
}

fn wire_supported(ty: ValueType) -> bool {
    matches!(
        ty,
        ValueType::String
            | ValueType::Int
            | ValueType::Bool
            | ValueType::F32
            | ValueType::StringList
    )
}

fn split_fields(input: &str) -> Result<Vec<&str>, CompileError> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut paren = 0i32;
    let mut bracket = 0i32;
    let mut brace = 0i32;
    let mut quote = false;
    let mut escaped = false;
    for (i, ch) in input.char_indices() {
        if quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                quote = false;
            }
            continue;
        }
        match ch {
            '"' => quote = true,
            '(' => paren += 1,
            ')' => paren -= 1,
            '[' => bracket += 1,
            ']' => bracket -= 1,
            '{' => brace += 1,
            '}' => brace -= 1,
            ',' if paren == 0 && bracket == 0 && brace == 0 => {
                let v = input[start..i].trim();
                if !v.is_empty() {
                    out.push(v);
                }
                start = i + 1;
            }
            _ => {}
        }
        if paren < 0 || bracket < 0 || brace < 0 {
            return Err(CompileError::Syntax(
                "unbalanced typed JSON response expression".into(),
            ));
        }
    }
    if quote || paren != 0 || bracket != 0 || brace != 0 {
        return Err(CompileError::Syntax(
            "unbalanced typed JSON response expression".into(),
        ));
    }
    let v = input[start..].trim();
    if !v.is_empty() {
        out.push(v);
    }
    Ok(out)
}
