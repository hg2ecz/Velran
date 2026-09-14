#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorizationMode {
    Owner { field: String },
    Authenticated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectAuthorization {
    pub object: String,
    pub mode: AuthorizationMode,
    pub allow_roles: Vec<String>,
}
