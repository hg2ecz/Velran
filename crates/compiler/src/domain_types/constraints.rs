use crate::diagnostics::CompileError;
use crate::lexer::tokenize;
use language_core::{ValidationKind, ValueType};

pub(super) fn parse(
    name: &str,
    base: ValueType,
    body: &str,
) -> Result<Vec<ValidationKind>, CompileError> {
    let tokens = tokenize(body)?;
    let mut constraints = Vec::new();
    let mut index = 0usize;
    while index < tokens.len() {
        if matches!(tokens[index].as_str(), ";" | ",") {
            index += 1;
            continue;
        }
        let (constraint, next) = parse_constraint(name, base, &tokens, index)?;
        if constraints
            .iter()
            .any(|existing| same_constraint_kind(existing, &constraint))
        {
            return Err(CompileError::Syntax(format!(
                "type `{name}` contains a duplicate constraint kind"
            )));
        }
        constraints.push(constraint);
        index = next;
    }
    if constraints.is_empty() {
        return Err(CompileError::Syntax(format!(
            "type `{name}` requires at least one constraint"
        )));
    }
    Ok(constraints)
}

fn parse_constraint(
    name: &str,
    base: ValueType,
    tokens: &[String],
    index: usize,
) -> Result<(ValidationKind, usize), CompileError> {
    match tokens.get(index).map(String::as_str) {
        Some("length") if base == ValueType::String => {
            let min = parse_usize(name, tokens.get(index + 1), "length minimum")?;
            let max = parse_usize(name, tokens.get(index + 2), "length maximum")?;
            if min > max || max > 4096 {
                return Err(CompileError::Syntax(format!(
                    "type `{name}` length must satisfy 0 <= min <= max <= 4096"
                )));
            }
            Ok((ValidationKind::Length { min, max }, index + 3))
        }
        Some("range") if base == ValueType::Int => {
            let min = parse_i64(name, tokens.get(index + 1), "range minimum")?;
            let max = parse_i64(name, tokens.get(index + 2), "range maximum")?;
            if min > max {
                return Err(CompileError::Syntax(format!(
                    "type `{name}` range minimum cannot exceed maximum"
                )));
            }
            Ok((ValidationKind::Range { min, max }, index + 3))
        }
        Some("pattern") if base == ValueType::String => {
            let regex = tokens
                .get(index + 1)
                .ok_or_else(|| CompileError::Syntax(format!("type `{name}` pattern expected")))?
                .clone();
            if regex.is_empty() || regex.len() > 256 || regex::Regex::new(&regex).is_err() {
                return Err(CompileError::Syntax(format!(
                    "type `{name}` contains an invalid pattern"
                )));
            }
            Ok((ValidationKind::Pattern { regex }, index + 2))
        }
        Some(kind) => Err(CompileError::Syntax(format!(
            "type `{name}` constraint `{kind}` does not match its base type"
        ))),
        None => Err(CompileError::Syntax(format!(
            "type `{name}` constraint expected"
        ))),
    }
}

fn same_constraint_kind(left: &ValidationKind, right: &ValidationKind) -> bool {
    matches!(
        (left, right),
        (ValidationKind::Length { .. }, ValidationKind::Length { .. })
            | (ValidationKind::Range { .. }, ValidationKind::Range { .. })
            | (
                ValidationKind::Pattern { .. },
                ValidationKind::Pattern { .. }
            )
    )
}

fn parse_usize(name: &str, raw: Option<&String>, label: &str) -> Result<usize, CompileError> {
    raw.ok_or_else(|| CompileError::Syntax(format!("type `{name}` {label} expected")))?
        .parse()
        .map_err(|_| CompileError::Syntax(format!("type `{name}` {label} must be an integer")))
}

fn parse_i64(name: &str, raw: Option<&String>, label: &str) -> Result<i64, CompileError> {
    raw.ok_or_else(|| CompileError::Syntax(format!("type `{name}` {label} expected")))?
        .parse()
        .map_err(|_| CompileError::Syntax(format!("type `{name}` {label} must be an integer")))
}

pub(super) fn apply_safe_defaults(base: ValueType, constraints: &mut Vec<ValidationKind>) {
    if base == ValueType::String
        && !constraints
            .iter()
            .any(|value| matches!(value, ValidationKind::Length { .. }))
    {
        constraints.insert(0, ValidationKind::Length { min: 0, max: 4096 });
    }
}
