use crate::diagnostics::CompileError;

pub(crate) fn validate_page_response_schema(
    statements: &[language_core::Statement],
    declared: Option<&str>,
    name: &str,
) -> Result<(), CompileError> {
    let mut found = Vec::new();
    collect_page_response_schemas(statements, &mut found);
    validate_response_schemas("page", name, declared, &found)
}

fn collect_page_response_schemas<'a>(
    statements: &'a [language_core::Statement],
    out: &mut Vec<Option<&'a str>>,
) {
    for statement in statements {
        match statement {
            language_core::Statement::ReturnTypedJson { schema, .. } => {
                out.push(Some(schema.as_str()))
            }
            language_core::Statement::ReturnJson(_)
            | language_core::Statement::ReturnJsonProjection(_) => out.push(None),
            language_core::Statement::Match { arms, .. } => {
                for arm in arms {
                    collect_page_response_schemas(&arm.statements, out);
                }
            }
            language_core::Statement::Resource { statements, .. } => {
                collect_page_response_schemas(statements, out)
            }
            _ => {}
        }
    }
}

pub(crate) fn validate_action_response_schema(
    statements: &[language_core::ActionStatement],
    declared: Option<&str>,
    name: &str,
) -> Result<(), CompileError> {
    let mut found = Vec::new();
    collect_action_response_schemas(statements, &mut found);
    validate_response_schemas("action", name, declared, &found)
}

fn collect_action_response_schemas<'a>(
    statements: &'a [language_core::ActionStatement],
    out: &mut Vec<Option<&'a str>>,
) {
    for statement in statements {
        match statement {
            language_core::ActionStatement::ReturnTypedJson { schema, .. } => {
                out.push(Some(schema.as_str()))
            }
            language_core::ActionStatement::ReturnJson(_)
            | language_core::ActionStatement::ReturnJsonProjection(_) => out.push(None),
            language_core::ActionStatement::Match { arms, .. } => {
                for arm in arms {
                    collect_action_response_schemas(&arm.statements, out);
                }
            }
            language_core::ActionStatement::Resource { statements, .. } => {
                collect_action_response_schemas(statements, out)
            }
            _ => {}
        }
    }
}

fn validate_response_schemas(
    kind: &str,
    name: &str,
    declared: Option<&str>,
    found: &[Option<&str>],
) -> Result<(), CompileError> {
    for actual in found {
        match (declared, actual) {
            (Some(expected), Some(actual)) if expected == *actual => {}
            (None, None) => {}
            (Some(expected), Some(actual)) => {
                return Err(CompileError::Syntax(format!(
                    "{kind} `{name}` returns typed JSON `{actual}` but declares `{expected}`"
                )));
            }
            (Some(expected), None) => {
                return Err(CompileError::Syntax(format!(
                    "{kind} `{name}` declares typed JSON `{expected}` but returns untyped `Json`"
                )));
            }
            (None, Some(actual)) => {
                return Err(CompileError::Syntax(format!(
                    "{kind} `{name}` returns typed JSON `{actual}` but declares untyped `Json`"
                )));
            }
        }
    }
    Ok(())
}
