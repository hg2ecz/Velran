use crate::declarations;
use crate::diagnostics::CompileError;
use crate::domain_validation;
use crate::lexer::tokenize;
use crate::module_namespace::qualify;
use crate::routes;
use crate::source_syntax::{matching_brace, read_ident};
use language_core::{FormField, FormSchema, Program, ValidationKind, ValidationRule, ValueType};

pub(super) fn parse_form_schemas(
    source: &str,
    namespace: &str,
    p: &mut Program,
) -> Result<(), CompileError> {
    let mut off = 0usize;
    while let Some(rel) = source[off..].find("form ") {
        let keyword = off + rel;
        if !declarations::is_top_level_declaration_at(source, keyword) {
            off = keyword + 5;
            continue;
        }
        if keyword > 0 {
            let prev = source.as_bytes()[keyword - 1];
            if prev.is_ascii_alphanumeric() || prev == b'_' {
                off = keyword + 5;
                continue;
            }
        }
        let start = keyword + 5;
        let Some(name) = read_ident(source, start) else {
            off = start;
            continue;
        };
        let after = start + name.len();
        let mut j = after;
        while j < source.len() && source.as_bytes()[j].is_ascii_whitespace() {
            j += 1;
        }
        if source.as_bytes().get(j) != Some(&b'{') {
            off = after;
            continue;
        }
        let symbol_name = qualify(namespace, &name);
        if p.forms.iter().any(|f| f.name == symbol_name) {
            return Err(CompileError::Syntax(format!("duplicate form `{name}`")));
        }
        let close = matching_brace(source, j)
            .ok_or_else(|| CompileError::Syntax(format!("form `{name}` unclosed")))?;
        let tokens = tokenize(&source[j + 1..close])?;
        let mut fields = Vec::new();
        let mut validations = Vec::new();
        let mut i = 0usize;
        while i < tokens.len() {
            if tokens[i] == "validate" {
                i += 1;
                while i < tokens.len() {
                    let (rule, next) = crate::validation_rules::parse_validation_rule(&tokens, i)?;
                    validations.push(rule);
                    i = next;
                }
                break;
            }
            let raw = &tokens[i];
            let field = routes::parse_typed_binding(&name, raw, namespace, p)?;
            validations.extend(domain_validation::rules_for_binding(raw, &field.name, namespace, p));
            if fields.iter().any(|f: &FormField| f.name == field.name) {
                return Err(CompileError::Syntax(format!(
                    "form `{name}` duplicate field `{}`",
                    field.name
                )));
            }
            fields.push(field);
            i += 1;
        }
        if fields.is_empty() {
            return Err(CompileError::Syntax(format!(
                "form `{name}` requires at least one field"
            )));
        }
        validate_form_rules(&name, &fields, &validations)?;
        p.forms.push(FormSchema {
            name: symbol_name,
            fields,
            validations,
        });
        off = close + 1;
    }
    Ok(())
}

fn validate_form_rules(
    name: &str,
    fields: &[FormField],
    rules: &[ValidationRule],
) -> Result<(), CompileError> {
    for r in rules {
        let f = fields.iter().find(|f| f.name == r.field).ok_or_else(|| {
            CompileError::Syntax(format!(
                "form `{name}` validation references unknown field `{}`",
                r.field
            ))
        })?;
        match &r.kind {
            ValidationKind::Length { .. } if f.ty == ValueType::String => {}
            ValidationKind::Range { .. } if f.ty == ValueType::Int => {}
            ValidationKind::Items { .. } if f.ty == ValueType::StringList => {}
            ValidationKind::Pattern { .. } if f.ty == ValueType::String => {}
            ValidationKind::SameAs { other } => {
                let o = fields.iter().find(|x| x.name == *other).ok_or_else(|| {
                    CompileError::Syntax(format!(
                        "form `{name}` same validation references unknown field `{other}`"
                    ))
                })?;
                if f.ty != o.ty {
                    return Err(CompileError::Syntax(format!(
                        "form `{name}` same validation requires matching field types `{}` and `{other}`",
                        r.field
                    )));
                }
                if matches!(
                    f.ty,
                    ValueType::Upload
                        | ValueType::Image
                        | ValueType::F32Array
                        | ValueType::StringList
                        | ValueType::StringDict
                ) {
                    return Err(CompileError::Syntax(format!(
                        "form `{name}` same validation does not support Upload/Image field `{}`",
                        r.field
                    )));
                }
            }
            ValidationKind::Length { .. } => {
                return Err(CompileError::Syntax(format!(
                    "form `{name}` length validation requires String field `{}`",
                    r.field
                )));
            }
            ValidationKind::Range { .. } => {
                return Err(CompileError::Syntax(format!(
                    "form `{name}` range validation requires Int field `{}`",
                    r.field
                )));
            }
            ValidationKind::Items { .. } => {
                return Err(CompileError::Syntax(format!(
                    "form `{name}` items validation requires List<String> field `{}`",
                    r.field
                )));
            }
            ValidationKind::Pattern { .. } => {
                return Err(CompileError::Syntax(format!(
                    "form `{name}` pattern validation requires String field `{}`",
                    r.field
                )));
            }
        }
    }
    Ok(())
}
