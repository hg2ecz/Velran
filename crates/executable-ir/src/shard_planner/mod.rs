#![forbid(unsafe_code)]

mod fingerprint;
mod planning;

use crate::{Capability, VerifiedHandler, VerifiedPureFunction};
use language_core::JsonSchema;
use std::collections::{BTreeMap, BTreeSet};

pub use planning::{DEFAULT_NATIVE_SHARD_BUCKETS, plan};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ShardId(String);

impl ShardId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub(crate) fn new(value: String) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedShard {
    pub(crate) id: ShardId,
    pub(crate) handlers: BTreeMap<String, VerifiedHandler>,
    pub(crate) pure_functions: BTreeMap<String, VerifiedPureFunction>,
    pub(crate) struct_schemas: BTreeMap<u16, JsonSchema>,
    pub(crate) capabilities: BTreeSet<Capability>,
    pub(crate) language_version: String,
    pub(crate) security_policy_version: String,
    pub(crate) executable_ir_version: u16,
    pub(crate) native_eligible: bool,
    pub(crate) interface_sha256: String,
    pub(crate) implementation_sha256: String,
}

impl VerifiedShard {
    pub fn id(&self) -> &ShardId {
        &self.id
    }
    pub fn handlers(&self) -> &BTreeMap<String, VerifiedHandler> {
        &self.handlers
    }
    pub fn pure_functions(&self) -> &BTreeMap<String, VerifiedPureFunction> {
        &self.pure_functions
    }
    pub fn struct_schemas(&self) -> &BTreeMap<u16, JsonSchema> {
        &self.struct_schemas
    }
    pub fn capabilities(&self) -> &BTreeSet<Capability> {
        &self.capabilities
    }
    pub fn language_version(&self) -> &str {
        &self.language_version
    }
    pub fn security_policy_version(&self) -> &str {
        &self.security_policy_version
    }
    pub fn executable_ir_version(&self) -> u16 {
        self.executable_ir_version
    }
    pub fn native_eligible(&self) -> bool {
        self.native_eligible
    }
    pub fn interface_sha256(&self) -> &str {
        &self.interface_sha256
    }
    pub fn implementation_sha256(&self) -> &str {
        &self.implementation_sha256
    }
    pub fn handler_id(&self, name: &str) -> Option<u32> {
        self.handlers
            .keys()
            .position(|candidate| candidate == name)
            .and_then(|index| u32::try_from(index).ok())
    }
}
