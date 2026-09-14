use crate::{FormField, HttpMethod, ValidationRule, ValueType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteSegment {
    Static(String),
    Param { name: String, ty: ValueType },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadField {
    pub name: String,
    pub destination: String,
    pub image: bool,
    pub publish: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteAuth {
    Public,
    Webhook(String),
    User,
    Mfa,
    Role(String),
    Permission { name: String, roles: Vec<String> },
    PermissionMfa { name: String, roles: Vec<String> },
}

impl Default for RouteAuth {
    fn default() -> Self {
        Self::Public
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicCachePolicy {
    pub ttl_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    pub name: String,
    pub method: HttpMethod,
    pub path: String,
    pub segments: Vec<RouteSegment>,
    pub query_fields: Vec<FormField>,
    pub form_fields: Vec<FormField>,
    pub form_schema: Option<String>,
    pub json_fields: Vec<FormField>,
    pub multipart_fields: Vec<FormField>,
    pub multipart_schema: Option<String>,
    pub upload: Option<UploadField>,
    pub validations: Vec<ValidationRule>,
    pub tenant_field: Option<String>,
    pub auth: RouteAuth,
    pub rate_policy: Option<String>,
    pub budget_profile: Option<String>,
    pub idempotent: bool,
    pub public_cache: Option<PublicCachePolicy>,
    pub invalidate_caches: Vec<String>,
    pub handler: String,
}
