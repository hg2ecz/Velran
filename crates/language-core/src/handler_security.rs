#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HandlerSecurityContract {
    pub required_permission: Option<String>,
    pub mfa_required: bool,
    pub critical_operation: Option<String>,
}
