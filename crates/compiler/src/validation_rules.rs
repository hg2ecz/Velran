use crate::diagnostics::CompileError;
use language_core::{ValidationKind, ValidationRule};

pub(super) fn parse_validation_rule(
    tokens: &[String],
    i: usize,
) -> Result<(ValidationRule, usize), CompileError> {
    let field = tokens
        .get(i)
        .ok_or_else(|| CompileError::Syntax("validation field expected".into()))?
        .clone();
    let kind = tokens
        .get(i + 1)
        .ok_or_else(|| CompileError::Syntax("validation kind expected".into()))?
        .as_str();
    match kind {
        "length" => {
            let a = tokens
                .get(i + 2)
                .ok_or_else(|| CompileError::Syntax("length minimum expected".into()))?;
            let b = tokens
                .get(i + 3)
                .ok_or_else(|| CompileError::Syntax("length maximum expected".into()))?;
            let min: usize = a
                .parse()
                .map_err(|_| CompileError::Syntax("length min must be integer".into()))?;
            let max: usize = b
                .parse()
                .map_err(|_| CompileError::Syntax("length max must be integer".into()))?;
            if min > max {
                return Err(CompileError::Syntax(
                    "validation min cannot exceed max".into(),
                ));
            }
            Ok((
                ValidationRule {
                    field,
                    kind: ValidationKind::Length { min, max },
                },
                i + 4,
            ))
        }
        "items" => {
            let a = tokens
                .get(i + 2)
                .ok_or_else(|| CompileError::Syntax("items minimum expected".into()))?;
            let b = tokens
                .get(i + 3)
                .ok_or_else(|| CompileError::Syntax("items maximum expected".into()))?;
            let min: usize = a
                .parse()
                .map_err(|_| CompileError::Syntax("items min must be integer".into()))?;
            let max: usize = b
                .parse()
                .map_err(|_| CompileError::Syntax("items max must be integer".into()))?;
            if min > max || max > 256 {
                return Err(CompileError::Syntax(
                    "items validation must satisfy 0 <= min <= max <= 256".into(),
                ));
            }
            Ok((
                ValidationRule {
                    field,
                    kind: ValidationKind::Items { min, max },
                },
                i + 4,
            ))
        }
        "range" => {
            let a = tokens
                .get(i + 2)
                .ok_or_else(|| CompileError::Syntax("range minimum expected".into()))?;
            let b = tokens
                .get(i + 3)
                .ok_or_else(|| CompileError::Syntax("range maximum expected".into()))?;
            let min: i64 = a
                .parse()
                .map_err(|_| CompileError::Syntax("range min must be integer".into()))?;
            let max: i64 = b
                .parse()
                .map_err(|_| CompileError::Syntax("range max must be integer".into()))?;
            if min > max {
                return Err(CompileError::Syntax(
                    "validation min cannot exceed max".into(),
                ));
            }
            Ok((
                ValidationRule {
                    field,
                    kind: ValidationKind::Range { min, max },
                },
                i + 4,
            ))
        }
        "pattern" => {
            let regex = tokens
                .get(i + 2)
                .ok_or_else(|| {
                    CompileError::Syntax(
                        "pattern validation requires a quoted regular expression".into(),
                    )
                })?
                .clone();
            if regex.is_empty() || regex.len() > 256 {
                return Err(CompileError::Syntax(
                    "pattern regular expression must be 1..256 bytes".into(),
                ));
            }
            regex::Regex::new(&regex).map_err(|_| {
                CompileError::Syntax(
                    "pattern validation contains an invalid regular expression".into(),
                )
            })?;
            Ok((
                ValidationRule {
                    field,
                    kind: ValidationKind::Pattern { regex },
                },
                i + 3,
            ))
        }
        "same" => {
            let other = tokens
                .get(i + 2)
                .ok_or_else(|| {
                    CompileError::Syntax("same validation requires another field".into())
                })?
                .clone();
            Ok((
                ValidationRule {
                    field,
                    kind: ValidationKind::SameAs { other },
                },
                i + 3,
            ))
        }
        _ => Err(CompileError::Syntax(format!(
            "unknown validation kind `{kind}`"
        ))),
    }
}
