use crate::diagnostics::CompileError;
use crate::expression::{infer_static_expr_type, parse_expr_in_namespace, validate_expr};
use crate::handler_types::StaticType;
use crate::module_namespace::resolve;
use crate::scalar_security::ScalarType;
use crate::type_semantics;
use language_core::{Expr, Program, ValueType};
use std::collections::HashMap;

pub(super) struct DomainRefinement {
    pub domain: u16,
    pub expr: Expr,
    pub static_type: StaticType,
}

pub(super) fn parse(
    text: &str,
    namespace: &str,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<Option<DomainRefinement>, CompileError> {
    let Some(rest) = text.strip_prefix("validate ") else {
        return Ok(None);
    };
    let Some(open) = rest.find('(') else {
        return syntax();
    };
    if !rest.ends_with(")?") {
        return syntax();
    }

    let type_name = rest[..open].trim();
    let expression_text = &rest[open + 1..rest.len() - 2];
    if type_name.is_empty() || expression_text.trim().is_empty() {
        return syntax();
    }

    let symbol = resolve(namespace, type_name);
    let (domain, definition) = program.domain_type_by_name(&symbol).ok_or_else(|| {
        CompileError::Syntax(format!(
            "validate target `{type_name}` must be a domain type"
        ))
    })?;
    let expr = parse_expr_in_namespace(expression_text.trim(), namespace, program)?;
    validate_expr(&expr, known, program)?;
    let source = infer_static_expr_type(&expr, known, program)?
        .scalar()
        .ok_or_else(|| {
            CompileError::Syntax("domain validation requires a scalar expression".into())
        })?;

    if let ValueType::Domain(source_domain) = source.value_type {
        if source_domain != domain {
            return Err(CompileError::security(
                "SEC-TYPE-002",
                format!(
                    "cannot refine `{}` into `{type_name}`: nominal domain types are not interchangeable",
                    type_semantics::display(program, source.value_type),
                ),
                Some(
                    "refine the original primitive/boundary value into the intended domain instead"
                        .into(),
                ),
            ));
        }
    } else if !type_semantics::represented_as(program, source.value_type, definition.base) {
        return Err(CompileError::security(
            "SEC-TYPE-002",
            format!(
                "cannot validate `{type_name}` from `{}`: expected {} representation",
                type_semantics::display(program, source.value_type),
                type_semantics::display(program, definition.base),
            ),
            Some(format!(
                "provide a {}-backed value and validate it explicitly",
                type_semantics::display(program, definition.base)
            )),
        ));
    }

    Ok(Some(DomainRefinement {
        domain,
        expr,
        static_type: StaticType::Scalar(ScalarType::validated_refinement(
            ValueType::Domain(domain),
            &source,
        )),
    }))
}

fn syntax<T>() -> Result<T, CompileError> {
    Err(CompileError::Syntax(
        "domain validation syntax is `validate Type(expression)?`".into(),
    ))
}
