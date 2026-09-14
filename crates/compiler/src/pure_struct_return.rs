use crate::diagnostics::CompileError;
use crate::expression::{infer_static_expr_type, parse_expr_in_namespace, validate_expr};
use crate::handler_types::StaticType;
use crate::module_namespace::resolve;
use crate::source_syntax::{is_identifier, matching_brace};
use language_core::{Program, TypedJsonField, ValueType};
use std::collections::{BTreeMap, HashMap};

pub(super) fn parse(
    raw: &str,
    namespace: &str,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<Option<(String, Vec<TypedJsonField>)>, CompileError> {
    let raw = raw.trim();
    let Some(open) = raw.find('{') else {
        return Ok(None);
    };
    let close = matching_brace(raw, open)
        .ok_or_else(|| CompileError::Syntax("pure struct literal is unclosed".into()))?;
    if !raw[close + 1..].trim().is_empty() {
        return Ok(None);
    }
    let type_name = raw[..open].trim();
    if type_name.is_empty() || !type_name.split("::").all(is_identifier) {
        return Ok(None);
    }
    let schema_name = if type_name.contains("::") {
        type_name.to_owned()
    } else {
        resolve(namespace, type_name)
    };
    let Some(schema) = program.json_schema(&schema_name) else {
        return Ok(None);
    };
    crate::visibility::require_access(program, &schema_name, namespace, "struct")?;

    let mut parsed = BTreeMap::<String, language_core::Expr>::new();
    for entry in split_fields(&raw[open + 1..close])? {
        let Some((name, expr_text)) = entry.split_once(':') else {
            return Err(CompileError::Syntax(format!(
                "pure struct field `{entry}` expected `name: expression`"
            )));
        };
        let name = name.trim();
        if !is_identifier(name) {
            return Err(CompileError::Syntax(format!(
                "invalid pure struct field `{name}`"
            )));
        }
        if parsed.contains_key(name) {
            return Err(CompileError::Syntax(format!(
                "duplicate pure struct field `{name}`"
            )));
        }
        let expr = parse_expr_in_namespace(expr_text.trim(), namespace, program)?;
        validate_expr(&expr, known, program)?;
        crate::visibility::validate_pure_expr_access(&expr, known, program, namespace)?;
        parsed.insert(name.to_owned(), expr);
    }
    if parsed.len() != schema.fields.len() {
        return Err(CompileError::Syntax(format!(
            "pure struct `{type_name}` must initialize exactly {} fields",
            schema.fields.len()
        )));
    }
    let mut fields = Vec::with_capacity(schema.fields.len());
    for field in &schema.fields {
        if !supported(field.ty) {
            return Err(CompileError::Syntax(format!(
                "pure struct field `{}.{}` has unsupported field type",
                type_name, field.name
            )));
        }
        crate::visibility::require_access(
            program,
            &format!("{schema_name}::{}", field.name),
            namespace,
            "field",
        )?;
        let expr = parsed.remove(&field.name).ok_or_else(|| {
            CompileError::Syntax(format!(
                "pure struct `{type_name}` missing field `{}`",
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
                "pure struct field `{}.{}` has the wrong type",
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
            "pure struct `{type_name}` has unknown field `{extra}`"
        )));
    }
    Ok(Some((schema_name, fields)))
}

fn supported(ty: ValueType) -> bool {
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
                "unbalanced pure struct literal".into(),
            ));
        }
    }
    if quote || paren != 0 || bracket != 0 || brace != 0 {
        return Err(CompileError::Syntax(
            "unbalanced pure struct literal".into(),
        ));
    }
    let v = input[start..].trim();
    if !v.is_empty() {
        out.push(v);
    }
    Ok(out)
}
