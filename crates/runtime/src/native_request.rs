use crate::request_binding::{
    decode_fields_into, decode_named_form_into, match_route, validate_route_inputs,
};
use language_core::{AppError, HttpMethod, Program, Route, Value, ValueType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeRequestValue {
    Int(i64),
    Bool(bool),
    String(String),
    Email(String),
    Url(String),
    Slug(String),
    DomainInt { domain: u16, value: i64 },
    DomainBool { domain: u16, value: bool },
    DomainString { domain: u16, value: String },
    Upload(Vec<u8>),
    Image(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRequest {
    pub handler_name: String,
    pub values: Vec<NativeRequestValue>,
}

pub fn prepare_native_request(
    program: &Program,
    method: HttpMethod,
    path: &str,
    query_pairs: &[(String, String)],
    form_pairs: &[(String, String)],
) -> Result<Option<NativeRequest>, AppError> {
    let (route, _) = match_route(program, method, path)?;
    prepare_native_request_for_route(program, route, method, path, query_pairs, form_pairs)
}

pub fn prepare_native_request_for_route(
    program: &Program,
    route: &Route,
    method: HttpMethod,
    path: &str,
    query_pairs: &[(String, String)],
    form_pairs: &[(String, String)],
) -> Result<Option<NativeRequest>, AppError> {
    let mut env = crate::request_binding::bind_route_path(program, route, path)?;
    decode_fields_into(program, &route.query_fields, query_pairs, &mut env)?;
    match method {
        HttpMethod::Get => validate_route_inputs(route, &env)?,
        HttpMethod::Post => {
            let body_schema = if route.json_fields.is_empty() {
                &route.form_fields
            } else {
                &route.json_fields
            };
            if route.form_schema.is_some() && route.json_fields.is_empty() {
                decode_named_form_into(program, route, form_pairs, &mut env)?;
            } else {
                decode_fields_into(program, body_schema, form_pairs, &mut env)?;
                validate_route_inputs(route, &env)?;
            }
        }
    }
    let params = match method {
        HttpMethod::Get => {
            &program
                .page(&route.handler)
                .ok_or(AppError::Internal)?
                .params
        }
        HttpMethod::Post => {
            &program
                .action(&route.handler)
                .ok_or(AppError::Internal)?
                .params
        }
    };
    let mut values = Vec::with_capacity(params.len());
    for param in params {
        let Some(value) = env.remove(&param.name) else {
            return Ok(None);
        };
        let Some(native) = to_native_value(program, param.ty, value)? else {
            return Ok(None);
        };
        values.push(native);
    }
    Ok(Some(NativeRequest {
        handler_name: route.handler.clone(),
        values,
    }))
}

fn to_native_value(
    program: &Program,
    ty: ValueType,
    value: Value,
) -> Result<Option<NativeRequestValue>, AppError> {
    Ok(Some(match (ty, value) {
        (ValueType::Int, Value::Int(v)) => NativeRequestValue::Int(v),
        (ValueType::Bool, Value::Bool(v)) => NativeRequestValue::Bool(v),
        (ValueType::String, Value::String(v)) => NativeRequestValue::String(v),
        (ValueType::Email, Value::Email(v)) => NativeRequestValue::Email(v),
        (ValueType::Url, Value::Url(v)) => NativeRequestValue::Url(v),
        (ValueType::Slug, Value::String(v)) => NativeRequestValue::Slug(v),
        (ValueType::Domain(id), Value::Int(v)) => NativeRequestValue::DomainInt {
            domain: id,
            value: v,
        },
        (ValueType::Domain(id), Value::Bool(v)) => NativeRequestValue::DomainBool {
            domain: id,
            value: v,
        },
        (ValueType::Domain(id), Value::String(v)) => NativeRequestValue::DomainString {
            domain: id,
            value: v,
        },
        (ValueType::Image, Value::Image(v)) => NativeRequestValue::Image(encode_image_ref(&v)),
        (ValueType::Domain(id), value) => {
            let _ = program.domain_type_by_id(id).ok_or(AppError::Internal)?;
            let _ = value;
            return Ok(None);
        }
        _ => return Ok(None),
    }))
}

fn encode_image_ref(v: &language_core::ImageRef) -> Vec<u8> {
    let mut out = vec![1u8];
    push_text(&mut out, &v.path);
    push_text(&mut out, &v.content_type);
    out.extend_from_slice(&v.width.to_le_bytes());
    out.extend_from_slice(&v.height.to_le_bytes());
    out.extend_from_slice(&v.bytes.to_le_bytes());
    out
}
fn push_text(out: &mut Vec<u8>, value: &str) {
    let len = u32::try_from(value.len()).expect("ImageRef strings are bounded below u32::MAX");
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(value.as_bytes());
}

#[cfg(test)]
mod native_image_abi_tests {
    use super::*;
    #[test]
    fn image_descriptor_is_versioned_and_bounded() {
        let image =
            language_core::ImageRef::new("media/a.png".into(), "image/png".into(), 10, 20, 30)
                .unwrap();
        let bytes = encode_image_ref(&image);
        assert_eq!(bytes[0], 1);
        assert!(bytes.len() < 1024);
    }
}

pub fn prepare_native_multipart_request_for_route(
    program: &Program,
    route: &Route,
    path: &str,
    text_pairs: &[(String, String)],
    upload_name: &str,
    upload_value: NativeRequestValue,
) -> Result<Option<NativeRequest>, AppError> {
    if route.method != HttpMethod::Post || route.multipart_schema.is_none() {
        return Err(AppError::Internal);
    }
    let mut env = crate::request_binding::bind_route_path(program, route, path)?;
    let text_schema: Vec<language_core::FormField> = route
        .multipart_fields
        .iter()
        .filter(|f| !matches!(f.ty, ValueType::Upload | ValueType::Image))
        .cloned()
        .collect();
    decode_fields_into(program, &text_schema, text_pairs, &mut env)?;
    validate_route_inputs(route, &env)?;
    let params = &program
        .action(&route.handler)
        .ok_or(AppError::Internal)?
        .params;
    let mut upload_value = Some(upload_value);
    let mut values = Vec::with_capacity(params.len());
    for param in params {
        if param.name == upload_name {
            let value = upload_value.take().ok_or(AppError::Internal)?;
            match (&value, param.ty) {
                (NativeRequestValue::Upload(_), ValueType::Upload)
                | (NativeRequestValue::Image(_), ValueType::Image) => values.push(value),
                _ => return Err(AppError::BadRequest),
            }
            continue;
        }
        let Some(value) = env.remove(&param.name) else {
            return Ok(None);
        };
        let Some(native) = to_native_value(program, param.ty, value)? else {
            return Ok(None);
        };
        values.push(native);
    }
    if upload_value.is_some() {
        return Err(AppError::Internal);
    }
    Ok(Some(NativeRequest {
        handler_name: route.handler.clone(),
        values,
    }))
}
