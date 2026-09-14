#![forbid(unsafe_code)]

mod emit;
mod lower;
mod numeric_support;
mod typed_lower;
mod typed_scalar_lower;

use executable_ir::Capability;
use executable_ir::shard_planner::VerifiedShard;
use runtime_abi::{
    RUNTIME_ABI_VERSION, STATUS_BAD_REQUEST, STATUS_BUDGET_EXCEEDED, STATUS_INTERNAL,
    STATUS_MEMORY_EXCEEDED, STATUS_OK, STATUS_OUTPUT_TOO_SMALL, STATUS_UNSUPPORTED, VALUE_BOOL,
    VALUE_HTML, VALUE_INT, VALUE_NONE, VALUE_TYPED_JSON,
};
use std::fmt;

pub const CODEGEN_VERSION: &str = "rust-aot-50-release-stabilization";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedRust {
    pub crate_name: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenError {
    EmptyShard,
    UnsupportedLowering { handler: String },
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyShard => f.write_str("cannot generate an empty shard"),
            Self::UnsupportedLowering { handler } => write!(
                f,
                "handler {handler} is not yet supported by the native scalar IR"
            ),
        }
    }
}
impl std::error::Error for CodegenError {}

pub fn generate(shard: &VerifiedShard) -> Result<GeneratedRust, CodegenError> {
    if shard.handlers().is_empty() {
        return Err(CodegenError::EmptyShard);
    }
    for (handler_name, handler) in shard.handlers() {
        if handler.native_scalar_body().is_none() {
            return Err(CodegenError::UnsupportedLowering {
                handler: handler_name.clone(),
            });
        }
    }
    Ok(emit::generate_source(shard))
}

fn capability_name(capability: &Capability) -> String {
    match capability {
        Capability::DbRead => "db.read".into(),
        Capability::DbWrite => "db.write".into(),
        Capability::SecurityAudit => "security.audit".into(),
        Capability::Network(target) => format!("net.{target}"),
        Capability::Permission(name) => format!("permission.{name}"),
        Capability::Mfa => "auth.mfa".into(),
        Capability::CriticalOperation(name) => format!("critical.{name}"),
    }
}

fn rust_ident(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert(0, '_');
    }
    if out.is_empty() { "shard".into() } else { out }
}

fn crate_ident(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut last_underscore = false;
    for ch in value.chars() {
        let ch = ch.to_ascii_lowercase();
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_underscore = false;
        } else if !last_underscore {
            out.push('_');
            last_underscore = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert(0, '_');
    }
    if out.is_empty() { "shard".into() } else { out }
}

fn local_ident(value: &str) -> String {
    format!("velran_local_{}", rust_ident(value))
}
pub(crate) fn pure_function_ident(value: &str) -> String {
    // Generated helper names are private Rust identifiers. Keep them lint-clean and
    // collision-resistant even when Velran namespaces contain separators/case.
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("velran_pure_{}_{:016x}", crate_ident(value), hash)
}

#[cfg(test)]
mod tests;
