use crate::{control_flow, diagnostics::CompileError};
use language_core::{
    ActionStatement, Expr, HtmlPart, HtmlTemplate, Program, Statement, TxStatement,
};

pub(super) fn action_has_object_auth(statements: &[ActionStatement]) -> bool {
    statements.iter().any(|s| match s {
        ActionStatement::Authorize(_) => true,
        ActionStatement::Resource { statements, .. } => action_has_object_auth(statements),
        ActionStatement::Match { arms, .. } => arms
            .iter()
            .any(|arm| action_has_object_auth(&arm.statements)),
        _ => false,
    })
}

pub(super) fn action_has_business_audit(statements: &[ActionStatement]) -> bool {
    statements.iter().any(|s| match s {
        ActionStatement::Transaction { statements, .. } => statements
            .iter()
            .any(|t| matches!(t, TxStatement::BusinessAudit(_))),
        ActionStatement::Resource { statements, .. } => action_has_business_audit(statements),
        ActionStatement::Match { arms, .. } => arms
            .iter()
            .any(|arm| action_has_business_audit(&arm.statements)),
        _ => false,
    })
}

pub(crate) fn expr_uses_request_state(e: &Expr) -> Option<&str> {
    match e {
        Expr::Variable(v)
            if matches!(
                v.as_str(),
                "csrfToken" | "authPrincipal" | "authMfaVerified"
            ) =>
        {
            Some(v.as_str())
        }
        Expr::Field { base, .. }
            if matches!(
                base.as_str(),
                "csrfToken" | "authPrincipal" | "authMfaVerified"
            ) =>
        {
            Some(base.as_str())
        }
        Expr::Slugify(inner) | Expr::Not(inner) => expr_uses_request_state(inner),
        Expr::CollectionIndex { index, .. } => expr_uses_request_state(index),
        Expr::F32ArrayNew { len, fill } => {
            expr_uses_request_state(len).or_else(|| expr_uses_request_state(fill))
        }
        Expr::Builtin { function, .. } if function.uses_request_state() => {
            Some(function.source_name())
        }
        Expr::Builtin { args, .. } => args.iter().find_map(expr_uses_request_state),
        Expr::Binary { left, right, .. } => {
            expr_uses_request_state(left).or_else(|| expr_uses_request_state(right))
        }
        _ => None,
    }
}

fn html_uses_request_state<'a>(t: &'a HtmlTemplate, p: &'a Program) -> Option<&'a str> {
    for part in &t.parts {
        let hit = match part {
            HtmlPart::EscapedExpr(e) | HtmlPart::Markdown(e) => expr_uses_request_state(e),
            HtmlPart::Image { image, alt } => {
                expr_uses_request_state(image).or_else(|| expr_uses_request_state(alt))
            }
            HtmlPart::RouteAttr { args, .. } => args.iter().find_map(expr_uses_request_state),
            HtmlPart::For { template, .. } | HtmlPart::IfSome { template, .. } => {
                html_uses_request_state(template, p)
            }
            HtmlPart::ComponentCall { component, args } => {
                args.iter().find_map(expr_uses_request_state).or_else(|| {
                    p.component(component)
                        .and_then(|d| html_uses_request_state(&d.template, p))
                })
            }
            HtmlPart::LayoutCall {
                layout,
                args,
                content,
            } => args
                .iter()
                .find_map(expr_uses_request_state)
                .or_else(|| html_uses_request_state(content, p))
                .or_else(|| {
                    p.layout(layout)
                        .and_then(|d| html_uses_request_state(&d.template, p))
                }),
            HtmlPart::Flash => Some("flash message"),
            HtmlPart::Text(_) | HtmlPart::ContentSlot => None,
        };
        if hit.is_some() {
            return hit;
        }
    }
    None
}

pub(super) fn validate_public_cache_statements(
    route: &str,
    statements: &[Statement],
    p: &Program,
) -> Result<(), CompileError> {
    for s in statements {
        let hit = match s {
            Statement::Let { expr, .. }
            | Statement::LetValidated { expr, .. }
            | Statement::Set { expr, .. }
            | Statement::ReturnJson(expr) => expr_uses_request_state(expr),
            Statement::While {
                condition,
                statements,
            } => expr_uses_request_state(condition)
                .or_else(|| control_flow::compute_uses_request_state(statements)),
            Statement::If {
                condition,
                statements,
            } => expr_uses_request_state(condition)
                .or_else(|| control_flow::compute_uses_request_state(statements)),
            Statement::Match { expr, arms, .. } => {
                if let Some(hit) = expr_uses_request_state(expr) {
                    Some(hit)
                } else {
                    for arm in arms {
                        validate_public_cache_statements(route, &arm.statements, p)?;
                    }
                    None
                }
            }
            Statement::F32ArraySet { index, value, .. } => {
                expr_uses_request_state(index).or_else(|| expr_uses_request_state(value))
            }
            Statement::StringDictSet { key, value, .. } => {
                expr_uses_request_state(key).or_else(|| expr_uses_request_state(value))
            }
            Statement::LetQuery { call, .. } => call.args.iter().find_map(expr_uses_request_state),
            Statement::LetOutboundStatus { .. } => None,
            Statement::PureCall { .. } => None,
            Statement::Authorize(_) => Some("authorization"),
            Statement::CanonicalSlug { .. } => Some("canonical redirect"),
            Statement::ReturnHtml(t) => html_uses_request_state(t, p),
            Statement::ReturnJsonProjection(_) | Statement::Fail(_) => None,
            Statement::ReturnTypedJson { fields, .. } => fields
                .iter()
                .find_map(|field| expr_uses_request_state(&field.expr)),
            Statement::Resource { statements, .. } => {
                validate_public_cache_statements(route, statements, p)?;
                None
            }
        };
        if let Some(name) = hit {
            return Err(CompileError::Syntax(format!(
                "route `{route}` public cache depends on request-specific value `{name}`"
            )));
        }
    }
    Ok(())
}
