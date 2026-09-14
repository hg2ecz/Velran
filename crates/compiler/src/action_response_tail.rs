use crate::diagnostics::CompileError;
use crate::expression::{parse_expr_in_namespace, validate_expr};
use crate::handler_types::StaticType;
use crate::public_projection::parse_public_projection;
use crate::response_security::{ResponseBoundary, validate_response_expression};
use crate::route_calls::parse_redirect_route_call;
use crate::source_syntax::{consume_return_tail, consume_tail_result, matching_paren};
use language_core::{ActionStatement, Program};
use std::collections::HashMap;

pub(crate) fn parse(
    name: &str,
    namespace: &str,
    body: &str,
    cursor: usize,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<Option<(ActionStatement, usize)>, CompileError> {
    if body[cursor..].starts_with("return Ok(json(") {
        let start = cursor + "return Ok(json(".len();
        let close = matching_paren(body, start - 1)
            .ok_or_else(|| CompileError::Syntax(format!("action `{name}` json return unclosed")))?;
        let raw = body[start..close].trim();
        let statement = if let Some(projection) = parse_public_projection(raw, known, program)? {
            ActionStatement::ReturnJsonProjection(projection)
        } else {
            let expr = parse_expr_in_namespace(raw, namespace, program)?;
            validate_expr(&expr, known, program)?;
            crate::visibility::validate_pure_expr_access(&expr, known, program, namespace)?;
            validate_response_expression(&expr, known, program, ResponseBoundary::Json)?;
            ActionStatement::ReturnJson(expr)
        };
        return Ok(Some((statement, consume_return_tail(body, close + 1)?)));
    }
    if body[cursor..].starts_with("return Ok(redirect(") {
        let start = cursor + "return Ok(redirect(".len();
        let close = matching_paren(body, start - 1)
            .ok_or_else(|| CompileError::Syntax(format!("action `{name}` redirect unclosed")))?;
        let route_call =
            parse_redirect_route_call(body[start..close].trim(), namespace, known, program)?;
        return Ok(Some((
            ActionStatement::ReturnRedirect(route_call),
            consume_return_tail(body, close + 1)?,
        )));
    }
    if body[cursor..].starts_with("return Ok(Json(") {
        let open = cursor + "return Ok".len();
        let close = matching_paren(body, open).ok_or_else(|| {
            CompileError::Syntax(format!("action `{name}` typed JSON return unclosed"))
        })?;
        let raw = body[open + 1..close].trim();
        let Some((schema, fields)) =
            crate::typed_json_response::parse(raw, namespace, known, program)?
        else {
            return Err(CompileError::Syntax(
                "typed JSON response expected `Json(Type { ... })`".into(),
            ));
        };
        return Ok(Some((
            ActionStatement::ReturnTypedJson { schema, fields },
            consume_return_tail(body, close + 1)?,
        )));
    }
    if body[cursor..].starts_with("Ok(Json(") {
        let open = cursor + "Ok".len();
        let close = matching_paren(body, open).ok_or_else(|| {
            CompileError::Syntax(format!(
                "{} `{}` typed JSON tail expression unclosed",
                "action", name
            ))
        })?;
        let raw = body[open + 1..close].trim();
        let Some((schema, fields)) =
            crate::typed_json_response::parse(raw, namespace, known, program)?
        else {
            return Err(CompileError::Syntax(
                "typed JSON response expected `Json(Type { ... })`".into(),
            ));
        };
        return Ok(Some((
            ActionStatement::ReturnTypedJson { schema, fields },
            consume_tail_result(body, close + 1)?,
        )));
    }
    if body[cursor..].starts_with("Ok(json(") {
        let start = cursor + "Ok(json(".len();
        let close = matching_paren(body, start - 1).ok_or_else(|| {
            CompileError::Syntax(format!("action `{name}` json tail expression unclosed"))
        })?;
        let raw = body[start..close].trim();
        let statement = if let Some(projection) = parse_public_projection(raw, known, program)? {
            ActionStatement::ReturnJsonProjection(projection)
        } else {
            let expr = parse_expr_in_namespace(raw, namespace, program)?;
            validate_expr(&expr, known, program)?;
            crate::visibility::validate_pure_expr_access(&expr, known, program, namespace)?;
            validate_response_expression(&expr, known, program, ResponseBoundary::Json)?;
            ActionStatement::ReturnJson(expr)
        };
        return Ok(Some((statement, consume_tail_result(body, close + 1)?)));
    }
    if body[cursor..].starts_with("Ok(redirect(") {
        let start = cursor + "Ok(redirect(".len();
        let close = matching_paren(body, start - 1).ok_or_else(|| {
            CompileError::Syntax(format!("action `{name}` redirect tail expression unclosed"))
        })?;
        let route_call =
            parse_redirect_route_call(body[start..close].trim(), namespace, known, program)?;
        return Ok(Some((
            ActionStatement::ReturnRedirect(route_call),
            consume_tail_result(body, close + 1)?,
        )));
    }
    Ok(None)
}
