use crate::handler_types::StaticType;
use crate::scalar_security::ScalarType;
use language_core::{
    CredentialPurpose, FunctionParam, Program, Route, RouteSegment, ValidationKind, ValueType,
};
use std::collections::HashMap;

pub(super) fn handler_input_types(
    handler: &str,
    params: &[FunctionParam],
    program: &Program,
) -> HashMap<String, StaticType> {
    let routes: Vec<&Route> = program
        .routes
        .iter()
        .filter(|route| route.handler == handler)
        .collect();

    params
        .iter()
        .map(|param| {
            let validated = !routes.is_empty()
                && routes
                    .iter()
                    .all(|route| route_validates(route, &param.name, param.ty));
            let active_tenant = validated
                && !routes.is_empty()
                && routes
                    .iter()
                    .all(|route| route.tenant_field.as_deref() == Some(param.name.as_str()));
            (
                param.name.clone(),
                external_param_type(param.ty, validated, active_tenant),
            )
        })
        .collect()
}

fn external_param_type(ty: ValueType, validated: bool, active_tenant: bool) -> StaticType {
    if ty == ValueType::Upload {
        return StaticType::Upload;
    }
    let scalar = if active_tenant {
        ScalarType::active_tenant(ty)
    } else if validated {
        ScalarType::validated(ty)
    } else {
        ScalarType::untrusted(ty)
    };
    StaticType::Scalar(scalar)
}

fn route_validates(route: &Route, name: &str, ty: ValueType) -> bool {
    let path_param = route.segments.iter().any(|segment| {
        matches!(segment, RouteSegment::Param { name: field, ty: field_ty } if field == name && *field_ty == ty)
    });
    let body_or_query_param = route
        .query_fields
        .iter()
        .chain(route.form_fields.iter())
        .chain(route.json_fields.iter())
        .any(|field| field.name == name && field.ty == ty);

    (path_param || body_or_query_param)
        && (scalar_decoder_validates(ty) || route_has_validation(route, name, ty))
}

fn scalar_decoder_validates(ty: ValueType) -> bool {
    if matches!(ty, ValueType::Domain(_)) {
        return true;
    }
    if let ValueType::Credential(purpose) = ty {
        return matches!(
            purpose,
            CredentialPurpose::Password
                | CredentialPurpose::ApiToken
                | CredentialPurpose::SessionToken
                | CredentialPurpose::PasswordResetToken
                | CredentialPurpose::CsrfToken
        );
    }
    !matches!(
        ty,
        ValueType::String
            | ValueType::StringList
            | ValueType::StringDict
            | ValueType::F32Array
            | ValueType::Upload
            | ValueType::Image
    )
}

fn route_has_validation(route: &Route, name: &str, ty: ValueType) -> bool {
    route.validations.iter().any(|rule| {
        rule.field == name
            && matches!(
                (&rule.kind, ty),
                (ValidationKind::Length { .. }, ValueType::String)
                    | (ValidationKind::Pattern { .. }, ValueType::String)
                    | (ValidationKind::Range { .. }, ValueType::Int)
                    | (ValidationKind::Items { .. }, ValueType::StringList)
                    | (ValidationKind::SameAs { .. }, _)
            )
    })
}
