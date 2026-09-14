use crate::VerifyError;
use language_core::{FunctionParam, Program, RouteSegment, ValidationKind, ValueType};

const MAX_NATIVE_INPUT_STRING_BYTES: usize = 1_048_576;

pub(crate) fn contract(
    program: &Program,
    handler: &str,
    params: &[FunctionParam],
) -> Result<bool, VerifyError> {
    if params.is_empty() {
        return Ok(true);
    }
    let routes: Vec<_> = program
        .routes
        .iter()
        .filter(|route| route.handler == handler)
        .collect();
    if routes.is_empty() {
        return Ok(false);
    }
    for route in routes {
        let mut expected: Vec<(String, ValueType)> = route
            .segments
            .iter()
            .filter_map(|segment| match segment {
                RouteSegment::Param { name, ty } => Some((name.clone(), *ty)),
                RouteSegment::Static(_) => None,
            })
            .collect();
        expected.extend(
            route
                .query_fields
                .iter()
                .map(|field| (field.name.clone(), field.ty)),
        );
        expected.extend(
            route
                .form_fields
                .iter()
                .map(|field| (field.name.clone(), field.ty)),
        );
        expected.extend(
            route
                .json_fields
                .iter()
                .map(|field| (field.name.clone(), field.ty)),
        );
        expected.extend(
            route
                .multipart_fields
                .iter()
                .map(|field| (field.name.clone(), field.ty)),
        );
        if route.multipart_schema.is_none() {
            if let Some(upload) = &route.upload {
                expected.push((
                    upload.name.clone(),
                    if upload.image {
                        ValueType::Image
                    } else {
                        ValueType::Upload
                    },
                ));
            }
        }
        if expected.len() != params.len() {
            return Err(invalid(handler, &route.name));
        }
        for ((name, ty), param) in expected.iter().zip(params) {
            if name != &param.name || *ty != param.ty {
                return Err(invalid(handler, &route.name));
            }
            if !native_type_allowed(program, route, name, *ty) {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn native_type_allowed(
    program: &Program,
    route: &language_core::Route,
    field: &str,
    ty: ValueType,
) -> bool {
    match ty {
        ValueType::Int | ValueType::Bool | ValueType::Email | ValueType::Url | ValueType::Slug => {
            true
        }
        ValueType::String => bounded_string(route, field),
        ValueType::Upload | ValueType::Image => true,
        ValueType::Domain(id) => domain_allowed(program, id),
        _ => false,
    }
}

fn domain_allowed(program: &Program, id: u16) -> bool {
    let Some(domain) = program.domain_type_by_id(id) else {
        return false;
    };
    match domain.base {
        ValueType::Int => domain.constraints.iter().all(|c| matches!(c, ValidationKind::Range { .. })),
        ValueType::String => domain.constraints.iter().all(|c| matches!(c, ValidationKind::Length { max, .. } if *max <= MAX_NATIVE_INPUT_STRING_BYTES)),
        ValueType::Bool => domain.constraints.is_empty(),
        _ => false,
    }
}

fn bounded_string(route: &language_core::Route, field: &str) -> bool {
    route.validations.iter().any(|rule| {
        rule.field == field && matches!(rule.kind, ValidationKind::Length { max, .. } if max <= MAX_NATIVE_INPUT_STRING_BYTES)
    })
}

fn invalid(handler: &str, route: &str) -> VerifyError {
    VerifyError::InvalidNativeInputContract {
        handler: handler.to_owned(),
        route: route.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use language_core::{
        DomainType, FormField, HttpMethod, PageBody, PageFunction, Route, RouteAuth, Statement,
        ValidationRule,
    };

    #[test]
    fn bounded_string_route_is_native_eligible() {
        let mut p = Program::default();
        p.pages.push(PageFunction {
            name: "search".into(),
            params: vec![FunctionParam::public("q", ValueType::String)],
            needs_db: false,
            effects: vec![],
            security: Default::default(),
            body: PageBody::Statements(vec![Statement::ReturnJson(language_core::Expr::Int(1))]),
        });
        p.routes.push(Route {
            name: "search".into(),
            method: HttpMethod::Get,
            path: "/".into(),
            segments: vec![],
            query_fields: vec![FormField {
                name: "q".into(),
                ty: ValueType::String,
            }],
            form_fields: vec![],
            form_schema: None,
            json_fields: vec![],
            multipart_fields: vec![],
            multipart_schema: None,
            upload: None,
            validations: vec![ValidationRule {
                field: "q".into(),
                kind: ValidationKind::Length { min: 0, max: 4096 },
            }],
            tenant_field: None,
            auth: RouteAuth::Public,
            rate_policy: None,
            budget_profile: None,
            idempotent: false,
            public_cache: None,
            invalidate_caches: vec![],
            handler: "search".into(),
        });
        assert_eq!(contract(&p, "search", &p.pages[0].params), Ok(true));
    }

    #[test]
    fn simple_nominal_domain_is_native_eligible() {
        let mut p = Program::default();
        p.domain_types.push(DomainType {
            name: "ArticleId".into(),
            base: ValueType::Int,
            constraints: vec![ValidationKind::Range {
                min: 1,
                max: 1_000_000,
            }],
        });
        assert!(domain_allowed(&p, 0));
    }

    #[test]
    fn regex_domain_stays_on_vm() {
        let mut p = Program::default();
        p.domain_types.push(DomainType {
            name: "Code".into(),
            base: ValueType::String,
            constraints: vec![ValidationKind::Pattern {
                regex: "^[A-Z]+$".into(),
            }],
        });
        assert!(!domain_allowed(&p, 0));
    }
}
