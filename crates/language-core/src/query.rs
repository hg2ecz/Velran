use crate::{CredentialPurpose, FunctionParam};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryCapability {
    Db,
    Transaction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryReturn {
    Void,
    /// Mutating query contract: exactly one row must be affected.
    Changed,
    One(String),
    Optional(String),
    List(String),
}

impl QueryReturn {
    pub fn model_name(&self) -> Option<&str> {
        match self {
            Self::Void | Self::Changed => None,
            Self::One(v) | Self::Optional(v) | Self::List(v) => Some(v),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationTarget {
    pub model: String,
    pub key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialLifecycleMode {
    ConsumeReset,
    RevokeSession,
    RotateSession,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialLifecycleTarget {
    pub mode: CredentialLifecycleMode,
    pub model: String,
    pub hash_field: String,
    pub expiry_field: Option<String>,
    pub hash_purpose: CredentialPurpose,
    pub replacement_hash_param: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantScopeTarget {
    pub model: String,
    pub field: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryFunction {
    pub name: String,
    pub capability: QueryCapability,
    pub params: Vec<FunctionParam>,
    pub return_type: QueryReturn,
    pub mutation_target: Option<MutationTarget>,
    pub tenant_scope: Option<TenantScopeTarget>,
    pub credential_lifecycle: Option<CredentialLifecycleTarget>,
    pub sql: String,
}
