use crate::diagnostics::CompileError;
use crate::handler_types::StaticType;
use crate::source_syntax::is_identifier;
use language_core::{AuthorizationMode, ObjectAuthorization, Program, ValueType};
use std::collections::HashMap;

pub(super) fn parse_and_refine_authorization(
    text: &str,
    known: &mut HashMap<String, StaticType>,
    program: &Program,
    handler: &str,
) -> Result<ObjectAuthorization, CompileError> {
    let parsed = parse_authorization(text, known, program, handler)?;
    known.insert(
        parsed.rule.object.clone(),
        StaticType::authorized_model(parsed.model_name),
    );
    Ok(parsed.rule)
}

struct ParsedAuthorization {
    rule: ObjectAuthorization,
    model_name: String,
}

fn parse_authorization(
    text: &str,
    known: &HashMap<String, StaticType>,
    program: &Program,
    handler: &str,
) -> Result<ParsedAuthorization, CompileError> {
    let words: Vec<&str> = text.split_whitespace().collect();
    validate_prefix(&words, handler)?;
    let object = words[1];
    if !is_identifier(object) {
        return Err(CompileError::Syntax(format!(
            "{handler} authorization object identifier is invalid"
        )));
    }
    let model_name = authorization_model_name(known, object, handler)?;
    let (mode, role_words) = parse_mode(&words[2..], program, &model_name, handler)?;
    let roles = parse_allowed_roles(role_words, handler)?;
    Ok(ParsedAuthorization {
        rule: ObjectAuthorization {
            object: object.into(),
            mode,
            allow_roles: roles,
        },
        model_name,
    })
}

fn validate_prefix(words: &[&str], handler: &str) -> Result<(), CompileError> {
    if words.len() < 3 || words[0] != "authorize" {
        return Err(CompileError::Syntax(format!(
            "{handler} authorization syntax is `authorize <record> owner <field> [or role <Role>]...` or `authorize <record> authenticated`"
        )));
    }
    Ok(())
}

fn parse_mode<'a>(
    words: &'a [&str],
    program: &Program,
    model_name: &str,
    handler: &str,
) -> Result<(AuthorizationMode, &'a [&'a str]), CompileError> {
    match words {
        ["authenticated", rest @ ..] => Ok((AuthorizationMode::Authenticated, rest)),
        ["owner", owner_field, rest @ ..] => {
            if !is_identifier(owner_field) {
                return Err(CompileError::Syntax(format!(
                    "{handler} authorization owner field is invalid"
                )));
            }
            validate_owner_field(program, model_name, owner_field, handler)?;
            Ok((
                AuthorizationMode::Owner {
                    field: (*owner_field).into(),
                },
                rest,
            ))
        }
        _ => Err(CompileError::Syntax(format!(
            "{handler} authorization expects `owner <field>` or `authenticated`"
        ))),
    }
}

fn authorization_model_name(
    known: &HashMap<String, StaticType>,
    object: &str,
    handler: &str,
) -> Result<String, CompileError> {
    match known.get(object) {
        Some(StaticType::Model(model)) => Ok(model.name.clone()),
        Some(StaticType::OptionalModel(_)) => Err(CompileError::Syntax(format!(
            "{handler} authorization requires a non-optional loaded object; handle absence before authorization"
        ))),
        _ => Err(CompileError::Syntax(format!(
            "{handler} authorization object `{object}` must be a loaded model"
        ))),
    }
}

fn validate_owner_field(
    program: &Program,
    model_name: &str,
    owner_field: &str,
    handler: &str,
) -> Result<(), CompileError> {
    let model = program
        .model(model_name)
        .ok_or_else(|| CompileError::UnknownModel(model_name.into()))?;
    let field = model
        .fields
        .iter()
        .find(|field| field.name == owner_field)
        .ok_or_else(|| CompileError::Syntax(format!(
            "{handler} authorization owner field `{owner_field}` does not exist on model `{model_name}`"
        )))?;
    if field.ty != ValueType::String {
        return Err(CompileError::Syntax(format!(
            "{handler} authorization owner field `{owner_field}` must be String"
        )));
    }
    Ok(())
}

fn parse_allowed_roles(words: &[&str], handler: &str) -> Result<Vec<String>, CompileError> {
    let mut roles = Vec::new();
    let mut cursor = 0;
    while cursor < words.len() {
        if cursor + 2 >= words.len()
            || words[cursor] != "or"
            || words[cursor + 1] != "role"
            || !is_identifier(words[cursor + 2])
        {
            return Err(CompileError::Syntax(format!(
                "{handler} authorization expected `or role <Role>`"
            )));
        }
        let role = words[cursor + 2].to_string();
        if roles.contains(&role) {
            return Err(CompileError::Syntax(format!(
                "{handler} duplicate authorization role `{role}`"
            )));
        }
        roles.push(role);
        cursor += 3;
    }
    Ok(roles)
}
