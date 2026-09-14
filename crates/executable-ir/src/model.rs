use crate::pure_model::VerifiedPureFunction;
use language_core::{ActionBody, EffectClass, JsonSchema, PageBody};
use std::collections::{BTreeMap, BTreeSet};

pub const EXECUTABLE_IR_VERSION: u16 = 25;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
    DbRead,
    DbWrite,
    SecurityAudit,
    Network(String),
    Permission(String),
    Mfa,
    CriticalOperation(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandlerManifest {
    pub name: String,
    pub effects: BTreeSet<EffectClass>,
    pub capabilities: BTreeSet<Capability>,
    pub route_names: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifiedHandlerBody {
    Page(PageBody),
    Action(ActionBody),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedHandler {
    pub(crate) manifest: HandlerManifest,
    pub(crate) body: VerifiedHandlerBody,
    pub(crate) native_scalar_body: Option<crate::VerifiedScalarBody>,
}

impl VerifiedHandler {
    pub fn manifest(&self) -> &HandlerManifest {
        &self.manifest
    }
    pub fn body(&self) -> &VerifiedHandlerBody {
        &self.body
    }
    pub fn native_scalar_body(&self) -> Option<&crate::VerifiedScalarBody> {
        self.native_scalar_body.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedExecutableProgram {
    pub(crate) pure_functions: BTreeMap<String, VerifiedPureFunction>,
    pub(crate) handlers: BTreeMap<String, VerifiedHandler>,
    pub(crate) struct_schemas: BTreeMap<u16, JsonSchema>,
    pub(crate) language_version: &'static str,
    pub(crate) security_policy_version: &'static str,
    pub(crate) executable_ir_version: u16,
}

impl VerifiedExecutableProgram {
    pub fn pure_functions(&self) -> &BTreeMap<String, VerifiedPureFunction> {
        &self.pure_functions
    }
    pub fn pure_function(&self, name: &str) -> Option<&VerifiedPureFunction> {
        self.pure_functions.get(name)
    }
    pub fn handlers(&self) -> &BTreeMap<String, VerifiedHandler> {
        &self.handlers
    }
    pub fn struct_schemas(&self) -> &BTreeMap<u16, JsonSchema> {
        &self.struct_schemas
    }
    pub fn handler(&self, name: &str) -> Option<&VerifiedHandler> {
        self.handlers.get(name)
    }
    pub fn language_version(&self) -> &str {
        self.language_version
    }
    pub fn security_policy_version(&self) -> &str {
        self.security_policy_version
    }
    pub fn executable_ir_version(&self) -> u16 {
        self.executable_ir_version
    }
}
