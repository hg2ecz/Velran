#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CriticalOperation {
    pub name: String,
    pub required_permission: Option<String>,
    pub mfa_required: bool,
    pub transaction_required: bool,
    pub audit_required: bool,
    pub idempotency_required: bool,
    pub required_security_event: Option<String>,
}
