#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductionPolicy {
    pub https_required: bool,
    pub debug_disabled: bool,
    pub hsts_required: bool,
    pub database_tls_required: bool,
}

impl ProductionPolicy {
    pub const STRICT: Self = Self {
        https_required: true,
        debug_disabled: true,
        hsts_required: true,
        database_tls_required: true,
    };
}
