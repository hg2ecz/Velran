use crate::diagnostics::CompileError;
use crate::handler_types::HandlerReturnKind;
use crate::module_namespace::resolve;
use crate::source_syntax::split_top_level;
use crate::type_resolution::resolve_annotated_value_type;
use language_core::{FunctionParam, HandlerSecurityContract, Program};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TypedRequestBoundary {
    Json,
    Query,
    Form,
    Multipart,
}

impl TypedRequestBoundary {
    fn label(self) -> &'static str {
        match self {
            Self::Json => "JSON",
            Self::Query => "query",
            Self::Form => "form",
            Self::Multipart => "multipart",
        }
    }
}

fn parse_typed_request_wrapper(ty: &str) -> Option<(TypedRequestBoundary, &str)> {
    for (prefix, boundary) in [
        ("Json<", TypedRequestBoundary::Json),
        ("Query<", TypedRequestBoundary::Query),
        ("Form<", TypedRequestBoundary::Form),
        ("Multipart<", TypedRequestBoundary::Multipart),
    ] {
        if let Some(inner) = ty.strip_prefix(prefix).and_then(|v| v.strip_suffix('>')) {
            let inner = inner.trim();
            if !inner.is_empty() {
                return Some((boundary, inner));
            }
        }
    }
    None
}

pub(super) fn parse_handler_params(
    function: &str,
    context: &str,
    input: &str,
    namespace: &str,
    p: &Program,
) -> Result<(Vec<FunctionParam>, bool, Vec<(String, String)>), CompileError> {
    let mut params = Vec::new();
    let mut needs_db = false;
    let mut request_aliases = Vec::new();
    for raw in split_top_level(input, ',') {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let (name, ty) = raw.split_once(':').ok_or_else(|| {
            CompileError::Syntax(format!(
                "function `{function}` parameter `{raw}` expected `name: Type`"
            ))
        })?;
        let name = name.trim();
        let ty = ty.trim();
        if matches!(name, "__flashKind" | "__flashMessage") {
            return Err(CompileError::Syntax(format!(
                "function `{function}` parameter name `{name}` is compiler-reserved"
            )));
        }
        if name == "ctx" && ty == context {
            continue;
        }
        if name == "db" && ty == "Db" {
            if needs_db {
                return Err(CompileError::Syntax(format!(
                    "function `{function}` duplicate Db capability"
                )));
            }
            needs_db = true;
            continue;
        }
        if let Some((boundary, inner)) = parse_typed_request_wrapper(ty) {
            match (boundary, context) {
                (TypedRequestBoundary::Json, "ActionContext" | "PageContext") => {}
                (TypedRequestBoundary::Json, _) => {
                    return Err(CompileError::Syntax(format!(
                        "function `{function}` Json<T> requires a page or action handler context"
                    )));
                }
                (TypedRequestBoundary::Query, "PageContext") => {}
                (TypedRequestBoundary::Form, "ActionContext") => {}
                (TypedRequestBoundary::Multipart, "ActionContext") => {}
                (TypedRequestBoundary::Query, _) => {
                    return Err(CompileError::Syntax(format!(
                        "function `{function}` Query<T> is only valid on #[page] handlers"
                    )));
                }
                (TypedRequestBoundary::Form, _) => {
                    return Err(CompileError::Syntax(format!(
                        "function `{function}` Form<T> is only valid on #[action] handlers"
                    )));
                }
                (TypedRequestBoundary::Multipart, _) => {
                    return Err(CompileError::Syntax(format!(
                        "function `{function}` Multipart<T> is only valid on #[action] handlers"
                    )));
                }
            }
            let schema_name = resolve(namespace, inner);
            let schema = p.json_schema(&schema_name).ok_or_else(|| {
                CompileError::Syntax(format!(
                    "function `{function}` references unknown {} struct `{inner}`",
                    boundary.label()
                ))
            })?;
            if boundary == TypedRequestBoundary::Multipart {
                let upload_count = schema
                    .fields
                    .iter()
                    .filter(|f| {
                        matches!(
                            f.ty,
                            language_core::ValueType::Upload | language_core::ValueType::Image
                        )
                    })
                    .count();
                if upload_count != 1 {
                    return Err(CompileError::Syntax(format!(
                        "function `{function}` Multipart<T> struct `{inner}` must contain exactly one Upload/Image field"
                    )));
                }
            }
            for field in &schema.fields {
                let allowed = match boundary {
                    TypedRequestBoundary::Multipart => matches!(
                        field.ty,
                        language_core::ValueType::String
                            | language_core::ValueType::Email
                            | language_core::ValueType::Url
                            | language_core::ValueType::Slug
                            | language_core::ValueType::Int
                            | language_core::ValueType::Bool
                            | language_core::ValueType::Upload
                            | language_core::ValueType::Image
                    ),
                    _ => !matches!(
                        field.ty,
                        language_core::ValueType::Upload | language_core::ValueType::Image
                    ),
                };
                if !allowed {
                    return Err(CompileError::Syntax(format!(
                        "function `{function}` {} field `{}` has unsupported request-boundary type",
                        boundary.label(),
                        field.name
                    )));
                }
                if params.iter().any(|p: &FunctionParam| p.name == field.name) {
                    return Err(CompileError::Syntax(format!(
                        "function `{function}` {} field `{}` conflicts with another parameter",
                        boundary.label(),
                        field.name
                    )));
                }
                params.push(FunctionParam::public(&field.name, field.ty));
                request_aliases.push((format!("{name}.{}", field.name), field.name.clone()));
            }
            continue;
        }
        let annotated = resolve_annotated_value_type(ty, namespace, p).ok_or_else(|| {
            CompileError::Syntax(format!(
                "function `{function}` unsupported parameter type `{ty}`"
            ))
        })?;
        if matches!(annotated.value_type, language_core::ValueType::Credential(purpose) if purpose.is_crypto_key())
        {
            return Err(CompileError::security(
                "SEC-A04-023",
                format!("web handler parameter `{function}.{name}` cannot accept cryptographic key material from the request boundary"),
                Some("load cryptographic keys from trusted secret storage or a secret-classified model/query boundary".into()),
            ));
        }
        if annotated.sensitivity != language_core::DataSensitivity::Public {
            return Err(CompileError::security(
                "SEC-DATA-009",
                format!(
                    "web handler parameter `{function}.{name}` cannot declare classified request-boundary types"
                ),
                Some("request parameters are trust-boundary inputs, not secret capabilities; load classified data from a model or an explicit trusted capability instead".into()),
            ));
        }
        params.push(FunctionParam::public(name, annotated.value_type));
    }
    Ok((params, needs_db, request_aliases))
}

pub(super) fn parse_handler_contract(
    kind: &str,
    name: &str,
    tail: &str,
    namespace: &str,
    program: &Program,
) -> Result<
    (
        HandlerReturnKind,
        HandlerSecurityContract,
        Vec<language_core::Effect>,
        Option<String>,
    ),
    CompileError,
> {
    let (tail, declared_effects) = match tail.rsplit_once(" uses ") {
        Some((before, raw)) => (
            before.trim_end(),
            crate::outbound_security::parse_handler_effects(name, raw, namespace, program)?,
        ),
        None => (tail, Vec::new()),
    };
    if tail.contains("requires") && tail.contains(" critical ") {
        return Err(CompileError::Syntax(format!(
            "handler `{name}` must use either `requires ...` or `critical <Operation>`, not both"
        )));
    }
    if let Some((return_tail, critical)) = tail.split_once(" critical ") {
        let (return_kind, response_schema) =
            parse_handler_return_kind(kind, name, return_tail, namespace, program)?;
        let security =
            crate::handler_security::parse_critical(name, critical.trim(), namespace, program)?;
        return Ok((return_kind, security, declared_effects, response_schema));
    }
    let (return_tail, requirements) = match tail.split_once("requires") {
        Some((before, after)) => (before, Some(after.trim())),
        None => (tail, None),
    };
    let (return_kind, response_schema) =
        parse_handler_return_kind(kind, name, return_tail, namespace, program)?;
    let security = requirements
        .map(|raw| crate::handler_security::parse_requirements(name, raw, namespace, program))
        .transpose()?
        .unwrap_or_default();
    Ok((return_kind, security, declared_effects, response_schema))
}

fn parse_handler_return_kind(
    kind: &str,
    name: &str,
    tail: &str,
    namespace: &str,
    program: &Program,
) -> Result<(HandlerReturnKind, Option<String>), CompileError> {
    let compact: String = tail.chars().filter(|c| !c.is_whitespace()).collect();
    match (kind, compact.as_str()) {
        ("page", "->Result<Html,PageError>") => return Ok((HandlerReturnKind::Html, None)),
        ("page", "->Result<Json,PageError>") => return Ok((HandlerReturnKind::Json, None)),
        ("action", "->Result<Redirect,PageError>") => {
            return Ok((HandlerReturnKind::Redirect, None));
        }
        ("action", "->Result<Json,PageError>") => return Ok((HandlerReturnKind::Json, None)),
        _ => {}
    }
    let prefix = "->Result<Json<";
    let suffix = ">,PageError>";
    if matches!(kind, "page" | "action") && compact.starts_with(prefix) && compact.ends_with(suffix)
    {
        let inner = &compact[prefix.len()..compact.len() - suffix.len()];
        let schema_name = resolve(namespace, inner);
        if program.json_schema(&schema_name).is_none() {
            return Err(CompileError::Syntax(format!(
                "{kind} `{name}` references unknown JSON response struct `{inner}`"
            )));
        }
        return Ok((HandlerReturnKind::Json, Some(schema_name)));
    }
    Err(CompileError::Syntax(format!(
        "{kind} `{name}` has unsupported return type `{}`",
        tail.trim()
    )))
}
