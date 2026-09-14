use crate::request_collections::{decode_string_list, group_fields};
use chrono::{DateTime, NaiveDate, Utc};
use language_core::{
    AppError, F32Value, FormFailure, FormField, FormFieldIssue, HttpMethod, ImageRef, Program,
    Route, RouteSegment, ValidationKind, Value, ValueType,
};
use rust_decimal::Decimal;
use std::collections::HashMap;
use uuid::Uuid;

pub fn route_meta_for_request<'a>(
    program: &'a Program,
    method: HttpMethod,
    path: &str,
) -> Result<&'a Route, AppError> {
    match_route(program, method, path).map(|(route, _)| route)
}

pub(crate) fn bind_route_path(
    program: &Program,
    route: &Route,
    path: &str,
) -> Result<HashMap<String, Value>, AppError> {
    let req: Vec<&str> = if path == "/" {
        vec![]
    } else {
        path.strip_prefix('/')
            .ok_or(AppError::BadRequest)?
            .split('/')
            .collect()
    };
    if route.segments.len() != req.len() {
        return Err(AppError::NotFound);
    }
    let mut env = HashMap::new();
    for (pattern, actual) in route.segments.iter().zip(&req) {
        match pattern {
            RouteSegment::Static(expected) if expected == actual => {}
            RouteSegment::Static(_) => return Err(AppError::NotFound),
            RouteSegment::Param { name, ty } => {
                let value =
                    decode_path_param(program, actual, *ty).map_err(|_| AppError::BadRequest)?;
                env.insert(name.clone(), value);
            }
        }
    }
    Ok(env)
}

pub(crate) fn match_route<'a>(
    program: &'a Program,
    method: HttpMethod,
    path: &str,
) -> Result<(&'a Route, HashMap<String, Value>), AppError> {
    let req: Vec<&str> = if path == "/" {
        vec![]
    } else {
        path.strip_prefix('/')
            .ok_or(AppError::BadRequest)?
            .split('/')
            .collect()
    };
    let mut bad = false;
    let mut other = false;
    for route in &program.routes {
        if route.segments.len() != req.len() {
            continue;
        }
        let mut env = HashMap::new();
        let mut matched = true;
        let mut badparam = false;
        for (pattern, actual) in route.segments.iter().zip(&req) {
            match pattern {
                RouteSegment::Static(x) if x == actual => {}
                RouteSegment::Static(_) => {
                    matched = false;
                    break;
                }
                RouteSegment::Param { name, ty } => match decode_path_param(program, actual, *ty) {
                    Ok(v) => {
                        env.insert(name.clone(), v);
                    }
                    Err(_) => {
                        matched = false;
                        badparam = true;
                        break;
                    }
                },
            }
        }
        if !matched {
            if badparam && route.method == method {
                bad = true;
            }
            continue;
        }
        if route.method != method {
            other = true;
            continue;
        }
        return Ok((route, env));
    }
    if bad {
        Err(AppError::BadRequest)
    } else if other {
        Err(AppError::MethodNotAllowed)
    } else {
        Err(AppError::NotFound)
    }
}
pub(crate) fn decode_named_form_into(
    program: &Program,
    route: &Route,
    pairs: &[(String, String)],
    env: &mut HashMap<String, Value>,
) -> Result<(), AppError> {
    let schema_name = route.form_schema.as_deref().ok_or(AppError::Internal)?;
    let grouped = group_fields(&route.form_fields, pairs)?;
    let mut issues = Vec::new();
    let mut values = Vec::new();
    for field in &route.form_fields {
        let supplied = grouped.get(&field.name);
        let display_value = match (field.ty, supplied) {
            (ValueType::Bool, None) => "false".into(),
            (_, Some(items)) => items.join(","),
            (_, None) => String::new(),
        };
        values.push((field.name.clone(), display_value));
        let value = match (field.ty, supplied) {
            (ValueType::StringList, Some(items)) => Some(decode_string_list(items)),
            (ValueType::StringList, None) => Some(Value::StringList(Vec::new())),
            (ValueType::Bool, None) => Some(Value::Bool(false)),
            (_, Some(items)) => match items.first() {
                Some(raw) => match decode_scalar(program, raw, field.ty) {
                    Ok(value) => Some(value),
                    Err(_) => {
                        issues.push(FormFieldIssue {
                            field: field.name.clone(),
                            code: "invalid_type".into(),
                        });
                        None
                    }
                },
                None => None,
            },
            (_, None) => {
                issues.push(FormFieldIssue {
                    field: field.name.clone(),
                    code: "required".into(),
                });
                None
            }
        };
        if let Some(value) = value {
            env.insert(field.name.clone(), value);
        }
    }
    if issues.is_empty() {
        for rule in &route.validations {
            let Some(value) = env.get(&rule.field) else {
                continue;
            };
            let ok = match (&rule.kind, value) {
                (ValidationKind::Length { min, max }, Value::String(v)) => {
                    v.chars().count() >= *min && v.chars().count() <= *max
                }
                (ValidationKind::Range { min, max }, Value::Int(v)) => v >= min && v <= max,
                (ValidationKind::Items { min, max }, Value::StringList(v)) => {
                    v.len() >= *min && v.len() <= *max
                }
                (ValidationKind::Pattern { regex }, Value::String(v)) => regex::Regex::new(regex)
                    .map(|re| re.is_match(v))
                    .unwrap_or(false),
                (ValidationKind::SameAs { other }, v) => env.get(other).is_some_and(|x| x == v),
                _ => false,
            };
            if !ok {
                let code = match &rule.kind {
                    ValidationKind::Length { .. } => "length",
                    ValidationKind::Range { .. } => "range",
                    ValidationKind::Items { .. } => "items",
                    ValidationKind::Pattern { .. } => "pattern",
                    ValidationKind::SameAs { .. } => "same",
                };
                issues.push(FormFieldIssue {
                    field: rule.field.clone(),
                    code: code.into(),
                });
            }
        }
    }
    if !issues.is_empty() {
        return Err(AppError::FormInvalid(FormFailure {
            schema: schema_name.into(),
            values,
            issues,
        }));
    }
    Ok(())
}
pub(crate) fn decode_fields_into(
    program: &Program,
    schema: &[FormField],
    pairs: &[(String, String)],
    env: &mut HashMap<String, Value>,
) -> Result<(), AppError> {
    let grouped = group_fields(schema, pairs)?;
    if grouped.len() != schema.len() {
        return Err(AppError::BadRequest);
    }
    for field in schema {
        let values = grouped.get(&field.name).ok_or(AppError::BadRequest)?;
        let value = if field.ty == ValueType::StringList {
            decode_string_list(values)
        } else {
            let raw = values.first().ok_or(AppError::BadRequest)?;
            decode_scalar(program, raw, field.ty)?
        };
        env.insert(field.name.clone(), value);
    }
    Ok(())
}
pub(crate) fn validate_route_inputs(
    route: &Route,
    env: &HashMap<String, Value>,
) -> Result<(), AppError> {
    for rule in &route.validations {
        let value = env.get(&rule.field).ok_or(AppError::BadRequest)?;
        match (&rule.kind, value) {
            (ValidationKind::Length { min, max }, Value::String(v))
                if v.chars().count() >= *min && v.chars().count() <= *max => {}
            (ValidationKind::Range { min, max }, Value::Int(v)) if v >= min && v <= max => {}
            (ValidationKind::Items { min, max }, Value::StringList(v))
                if v.len() >= *min && v.len() <= *max => {}
            (ValidationKind::Pattern { regex }, Value::String(v))
                if regex::Regex::new(regex)
                    .map(|re| re.is_match(v))
                    .unwrap_or(false) => {}
            (ValidationKind::SameAs { other }, v) if env.get(other).is_some_and(|x| x == v) => {}
            _ => return Err(AppError::BadRequest),
        }
    }
    Ok(())
}
fn decode_path_param(program: &Program, raw: &str, ty: ValueType) -> Result<Value, AppError> {
    decode_scalar(program, &percent_decode(raw, false)?, ty)
}
use crate::scalars::{is_canonical_slug, normalize_email, normalize_url};
use language_core::CredentialPurpose;

pub(crate) fn decode_scalar(
    program: &Program,
    raw: &str,
    ty: ValueType,
) -> Result<Value, AppError> {
    if let ValueType::Domain(id) = ty {
        let base = program
            .domain_type_by_id(id)
            .ok_or(AppError::Internal)?
            .base;
        let value = decode_scalar(program, raw, base)?;
        return crate::domain_values::validate(program, id, value);
    }
    if let ValueType::Credential(purpose) = ty {
        return decode_credential(raw, purpose);
    }
    match ty {
        ValueType::String => Ok(Value::String(raw.into())),
        ValueType::Email => normalize_email(raw)
            .map(Value::Email)
            .ok_or(AppError::BadRequest),
        ValueType::Url => normalize_url(raw)
            .map(Value::Url)
            .ok_or(AppError::BadRequest),
        ValueType::Slug => {
            if is_canonical_slug(raw) {
                Ok(Value::String(raw.into()))
            } else {
                Err(AppError::BadRequest)
            }
        }
        ValueType::Int => raw
            .parse::<i64>()
            .map(Value::Int)
            .map_err(|_| AppError::BadRequest),
        ValueType::F32Array | ValueType::StringList | ValueType::StringDict => {
            Err(AppError::BadRequest)
        }
        ValueType::F32 => raw
            .parse::<f32>()
            .ok()
            .and_then(F32Value::new)
            .map(Value::F32)
            .ok_or(AppError::BadRequest),
        ValueType::Bool => match raw {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            _ => Err(AppError::BadRequest),
        },
        ValueType::Date => NaiveDate::parse_from_str(raw, "%Y-%m-%d")
            .map(Value::Date)
            .map_err(|_| AppError::BadRequest),
        ValueType::DateTime => DateTime::parse_from_rfc3339(raw)
            .map(|v| Value::DateTime(v.with_timezone(&Utc)))
            .map_err(|_| AppError::BadRequest),
        ValueType::Uuid => Uuid::parse_str(raw)
            .map(Value::Uuid)
            .map_err(|_| AppError::BadRequest),
        ValueType::Decimal => Decimal::from_str_exact(raw)
            .map(Value::Decimal)
            .map_err(|_| AppError::BadRequest),
        ValueType::Image => ImageRef::parse(raw)
            .map(Value::Image)
            .ok_or(AppError::BadRequest),
        ValueType::Enum(enum_id) => {
            let def = program.enum_by_id(enum_id).ok_or(AppError::Internal)?;
            if def.variants.iter().any(|v| v == raw) {
                Ok(Value::Enum {
                    enum_id,
                    variant: raw.into(),
                })
            } else {
                Err(AppError::BadRequest)
            }
        }
        ValueType::Upload => Err(AppError::BadRequest),
        ValueType::Credential(_) => {
            unreachable!("credential types are decoded before scalar decoding")
        }
        ValueType::Domain(_) => unreachable!("domain types are unwrapped before scalar decoding"),
    }
}

fn decode_credential(raw: &str, purpose: CredentialPurpose) -> Result<Value, AppError> {
    let valid_length = match purpose {
        CredentialPurpose::Password => (12..=1024).contains(&raw.len()),
        CredentialPurpose::PasswordHash => (32..=512).contains(&raw.len()),
        CredentialPurpose::ApiToken => (16..=4096).contains(&raw.len()),
        CredentialPurpose::SessionToken
        | CredentialPurpose::PasswordResetToken
        | CredentialPurpose::CsrfToken => {
            raw.len() == 64 && raw.bytes().all(|byte| byte.is_ascii_hexdigit())
        }
        CredentialPurpose::SessionTokenHash
        | CredentialPurpose::PasswordResetTokenHash
        | CredentialPurpose::CsrfTokenHash => false,
        CredentialPurpose::CryptoKey => (16..=4096).contains(&raw.len()),
        CredentialPurpose::SigningKeyWebhook
        | CredentialPurpose::RetiringSigningKeyWebhook
        | CredentialPurpose::RetiredSigningKeyWebhook
        | CredentialPurpose::VerificationKeyWebhook
        | CredentialPurpose::RetiringVerificationKeyWebhook
        | CredentialPurpose::RetiredVerificationKeyWebhook
        | CredentialPurpose::EncryptionKeyUserData
        | CredentialPurpose::RetiringEncryptionKeyUserData
        | CredentialPurpose::RetiredEncryptionKeyUserData => false,
    };
    if !valid_length
        || raw
            .bytes()
            .any(|byte| byte == 0 || byte == b'\r' || byte == b'\n')
    {
        return Err(AppError::BadRequest);
    }
    Ok(Value::String(raw.into()))
}

pub fn decode_urlencoded_limited(
    body: &[u8],
    max_fields: usize,
    max_field_bytes: usize,
) -> Result<Vec<(String, String)>, AppError> {
    let text = std::str::from_utf8(body).map_err(|_| AppError::BadRequest)?;
    if text.is_empty() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for pair in text.split('&') {
        if out.len() >= max_fields {
            return Err(AppError::BadRequest);
        }
        if pair.len() > max_field_bytes.saturating_mul(3).saturating_add(8) {
            return Err(AppError::BadRequest);
        }
        let (n, v) = pair.split_once('=').unwrap_or((pair, ""));
        let n = percent_decode(n, true)?;
        let v = percent_decode(v, true)?;
        if n.is_empty()
            || n.len() > max_field_bytes
            || v.len() > max_field_bytes
            || n.bytes().any(|b| b < 0x20 || b == 0x7f)
            || v.bytes().any(|b| b == 0)
        {
            return Err(AppError::BadRequest);
        }
        out.push((n, v));
    }
    Ok(out)
}
pub fn decode_urlencoded(body: &[u8]) -> Result<Vec<(String, String)>, AppError> {
    decode_urlencoded_limited(body, 64, 8192)
}
fn percent_decode(raw: &str, plus: bool) -> Result<String, AppError> {
    let b = raw.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' => {
                let hi = *b.get(i + 1).ok_or(AppError::BadRequest)?;
                let lo = *b.get(i + 2).ok_or(AppError::BadRequest)?;
                let x = (hex(hi).ok_or(AppError::BadRequest)? << 4)
                    | hex(lo).ok_or(AppError::BadRequest)?;
                if x == 0 || (!plus && x == b'/') {
                    return Err(AppError::BadRequest);
                }
                out.push(x);
                i += 3
            }
            b'+' if plus => {
                out.push(b' ');
                i += 1
            }
            x if x == 0 || (!plus && x == b'/') => return Err(AppError::BadRequest),
            x => {
                out.push(x);
                i += 1
            }
        }
    }
    String::from_utf8(out).map_err(|_| AppError::BadRequest)
}
fn hex(v: u8) -> Option<u8> {
    match v {
        b'0'..=b'9' => Some(v - b'0'),
        b'a'..=b'f' => Some(v - b'a' + 10),
        b'A'..=b'F' => Some(v - b'A' + 10),
        _ => None,
    }
}
