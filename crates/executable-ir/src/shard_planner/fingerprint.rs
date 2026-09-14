use crate::{
    Capability, NativeInputType, VerifiedHandler, VerifiedHandlerBody, VerifiedPureFunction,
};
use language_core::{EffectClass, JsonSchema};
use sha2::{Digest, Sha256};

/// Hashes only the semantic contract visible outside the handler implementation.
/// Formatting/comments and private statement bodies cannot change this value.
pub(crate) fn interface_sha256(handler: &VerifiedHandler) -> String {
    let mut out = Vec::new();
    field(&mut out, "handler", &handler.manifest().name);
    field(
        &mut out,
        "kind",
        match handler.body() {
            VerifiedHandlerBody::Page(_) => "page",
            VerifiedHandlerBody::Action(_) => "action",
        },
    );
    for route in &handler.manifest().route_names {
        field(&mut out, "route", route);
    }
    for effect in &handler.manifest().effects {
        encode_effect(&mut out, *effect);
    }
    for capability in &handler.manifest().capabilities {
        encode_capability(&mut out, capability);
    }
    match handler.native_scalar_body() {
        Some(body) => {
            field(&mut out, "native", "scalar-v2-inputs");
            for input in body.inputs() {
                field(&mut out, "input", &input.name);
                encode_input_type(&mut out, &input.ty);
            }
        }
        None => field(&mut out, "native", "unsupported"),
    }
    hex_sha256(&out)
}

/// Hashes the verified semantic implementation rather than the source text.
/// The EIR version is part of the cache identity, so Debug layout changes to this
/// compiler-owned IR must be accompanied by an EIR version bump.
pub(crate) fn implementation_sha256(handler: &VerifiedHandler) -> String {
    let mut out = Vec::new();
    field(&mut out, "interface", &interface_sha256(handler));
    match handler.native_scalar_body() {
        Some(body) => field(&mut out, "verified-native-body", &format!("{body:?}")),
        None => field(
            &mut out,
            "verified-nonnative-body",
            &format!("{:?}", handler.body()),
        ),
    }
    hex_sha256(&out)
}

pub(crate) fn struct_schema_sha256(schema: &JsonSchema) -> String {
    let mut out = Vec::new();
    field(
        &mut out,
        "verified-pure-struct-schema",
        &format!("{schema:?}"),
    );
    hex_sha256(&out)
}

pub(crate) fn pure_function_sha256(function: &VerifiedPureFunction) -> String {
    let mut out = Vec::new();
    field(&mut out, "verified-pure-function", &format!("{function:?}"));
    hex_sha256(&out)
}

pub(crate) fn shard_sha256<'a>(
    label: &str,
    values: impl Iterator<Item = (&'a str, &'a str)>,
) -> String {
    let mut out = Vec::new();
    field(&mut out, "kind", label);
    for (name, value) in values {
        field(&mut out, "handler", name);
        field(&mut out, label, value);
    }
    hex_sha256(&out)
}

fn encode_effect(out: &mut Vec<u8>, effect: EffectClass) {
    field(out, "effect", effect.source_name());
}

fn encode_capability(out: &mut Vec<u8>, capability: &Capability) {
    match capability {
        Capability::DbRead => field(out, "cap", "db.read"),
        Capability::DbWrite => field(out, "cap", "db.write"),
        Capability::SecurityAudit => field(out, "cap", "security.audit"),
        Capability::Network(target) => field(out, "cap.net", target),
        Capability::Permission(name) => field(out, "cap.permission", name),
        Capability::Mfa => field(out, "cap", "auth.mfa"),
        Capability::CriticalOperation(name) => field(out, "cap.critical", name),
    }
}

fn encode_input_type(out: &mut Vec<u8>, ty: &NativeInputType) {
    match ty {
        NativeInputType::String => field(out, "type", "String"),
        NativeInputType::StringList => field(out, "type", "internal-string-list-ref"),
        NativeInputType::Struct(id) => {
            field(out, "type", "internal-struct-ref");
            field(out, "schema", &id.to_string());
        }
        NativeInputType::Email => field(out, "type", "Email"),
        NativeInputType::Url => field(out, "type", "Url"),
        NativeInputType::Slug => field(out, "type", "Slug"),
        NativeInputType::Int => field(out, "type", "i64"),
        NativeInputType::F32Array => field(out, "type", "internal-f32-array-ref"),
        NativeInputType::Bool => field(out, "type", "bool"),
        NativeInputType::DomainInt { domain, ranges } => {
            field(out, "type", "domain-int");
            field(out, "domain", &domain.to_string());
            for (min, max) in ranges {
                field(out, "range", &format!("{min}:{max}"));
            }
        }
        NativeInputType::DomainBool { domain } => {
            field(out, "type", "domain-bool");
            field(out, "domain", &domain.to_string());
        }
        NativeInputType::Upload => field(out, "type", "Upload"),
        NativeInputType::Image => field(out, "type", "Image"),
        NativeInputType::DomainString { domain, lengths } => {
            field(out, "type", "domain-string");
            field(out, "domain", &domain.to_string());
            for (min, max) in lengths {
                field(out, "length", &format!("{min}:{max}"));
            }
        }
    }
}

fn field(out: &mut Vec<u8>, name: &str, value: &str) {
    out.extend_from_slice(name.as_bytes());
    out.push(0);
    out.extend_from_slice(value.len().to_string().as_bytes());
    out.push(b':');
    out.extend_from_slice(value.as_bytes());
    out.push(0xff);
}

fn hex_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
