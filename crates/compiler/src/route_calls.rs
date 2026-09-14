use crate::diagnostics::CompileError;
use crate::expression::{infer_expr_type, parse_expr_in_namespace, validate_expr};
use crate::handler_types::StaticType;
use crate::source_syntax::{is_identifier, matching_paren, split_top_level};
use crate::type_semantics::display;
use language_core::{HttpMethod, Program, RouteCall, RouteSegment, ValueType};
use std::collections::HashMap;

pub(super) fn parse_redirect_route_call(
    raw: &str,
    namespace: &str,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<RouteCall, CompileError> {
    let raw = raw.trim();
    let open = raw.find('(').ok_or_else(|| typed_redirect_syntax_error())?;
    let route_name = raw[..open].trim();
    if !is_identifier(route_name) {
        return Err(typed_redirect_syntax_error());
    }
    let close = matching_paren(raw, open).ok_or_else(typed_redirect_syntax_error)?;
    if !raw[close + 1..].trim().is_empty() {
        return Err(typed_redirect_syntax_error());
    }
    let route = program
        .routes
        .iter()
        .find(|route| route.name == route_name)
        .ok_or_else(|| CompileError::security(
            "SEC-A01-007",
            format!("redirect references unknown route `{route_name}`"),
            Some("redirect to a declared GET route by name, for example `redirect(AccountView(user.id))`".into()),
        ))?;
    if route.method != HttpMethod::Get {
        return Err(CompileError::security(
            "SEC-A01-008",
            format!("redirect target `{route_name}` is not a GET route"),
            Some("redirects must target a GET route; submit mutations through POST and redirect to the resulting GET page".into()),
        ));
    }
    let expected = redirect_argument_types(route);
    let pieces = split_top_level(&raw[open + 1..close], ',');
    let pieces = if pieces.len() == 1 && pieces[0].trim().is_empty() {
        Vec::new()
    } else {
        pieces
    };
    if pieces.len() != expected.len() {
        return Err(CompileError::security(
            "SEC-A01-009",
            format!(
                "redirect route `{route_name}` expects {} arguments, got {}",
                expected.len(),
                pieces.len()
            ),
            Some("pass the route path parameters first, followed by its query parameters".into()),
        ));
    }
    let mut args = Vec::with_capacity(expected.len());
    for (piece, expected_type) in pieces.into_iter().zip(expected) {
        let expression = parse_expr_in_namespace(piece.trim(), namespace, program)?;
        validate_expr(&expression, known, program)?;
        let actual_type = infer_expr_type(&expression, known, program)?;
        if actual_type != expected_type {
            return Err(CompileError::security(
                "SEC-A01-010",
                format!(
                    "redirect route `{route_name}` expects `{}`, got `{}`",
                    display(program, expected_type),
                    display(program, actual_type)
                ),
                Some("pass values matching the declared route parameter types".into()),
            ));
        }
        args.push(expression);
    }
    Ok(RouteCall {
        route: route_name.into(),
        args,
    })
}

fn redirect_argument_types(route: &language_core::Route) -> Vec<ValueType> {
    let mut expected: Vec<ValueType> = route
        .segments
        .iter()
        .filter_map(|segment| match segment {
            RouteSegment::Param { ty, .. } => Some(*ty),
            RouteSegment::Static(_) => None,
        })
        .collect();
    expected.extend(route.query_fields.iter().map(|field| field.ty));
    expected
}

fn typed_redirect_syntax_error() -> CompileError {
    CompileError::security(
        "SEC-A01-011",
        "redirect target must be a typed route call, not a string URL",
        Some("use `redirect(RouteName(args...))`; Velran builds the local URL from the route declaration".into()),
    )
}
