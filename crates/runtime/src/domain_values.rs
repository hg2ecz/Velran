use language_core::{AppError, Program, ValidationKind, Value};

pub(crate) fn validate(program: &Program, domain: u16, value: Value) -> Result<Value, AppError> {
    let definition = program
        .domain_type_by_id(domain)
        .ok_or(AppError::Internal)?;
    if definition
        .constraints
        .iter()
        .all(|constraint| matches_constraint(constraint, &value))
    {
        Ok(value)
    } else {
        Err(AppError::BadRequest)
    }
}

fn matches_constraint(constraint: &ValidationKind, value: &Value) -> bool {
    match (constraint, value) {
        (ValidationKind::Length { min, max }, Value::String(text)) => {
            let len = text.chars().count();
            len >= *min && len <= *max
        }
        (ValidationKind::Range { min, max }, Value::Int(number)) => number >= min && number <= max,
        (ValidationKind::Items { .. }, _) => false,
        (ValidationKind::Pattern { regex }, Value::String(text)) => {
            regex::Regex::new(regex).is_ok_and(|compiled| compiled.is_match(text))
        }
        (ValidationKind::SameAs { .. }, _) => false,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use language_core::{DomainType, ValidationKind, ValueType};

    fn program_with_domain(base: ValueType, constraints: Vec<ValidationKind>) -> Program {
        Program {
            domain_types: vec![DomainType {
                name: "Example".into(),
                base,
                constraints,
            }],
            ..Program::default()
        }
    }

    #[test]
    fn accepts_value_that_satisfies_domain_contract() {
        let program = program_with_domain(
            ValueType::Int,
            vec![ValidationKind::Range { min: 1, max: 10 }],
        );
        assert_eq!(validate(&program, 0, Value::Int(7)), Ok(Value::Int(7)));
    }

    #[test]
    fn rejects_value_outside_domain_contract() {
        let program = program_with_domain(
            ValueType::String,
            vec![ValidationKind::Length { min: 3, max: 5 }],
        );
        assert_eq!(
            validate(&program, 0, Value::String("xx".into())),
            Err(AppError::BadRequest)
        );
    }
}
