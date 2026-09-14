#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EffectClass {
    Pure,
    TimeRead,
    Crypto,
    DbRead,
    DbWrite,
    CacheRead,
    CacheWrite,
    OutboundHttp,
    UploadRead,
    StorageRead,
    StorageWrite,
    AuthRead,
    SessionWrite,
    SecurityAudit,
}

impl EffectClass {
    pub const fn source_name(self) -> &'static str {
        match self {
            Self::Pure => "PURE",
            Self::TimeRead => "TIME_READ",
            Self::Crypto => "CRYPTO",
            Self::DbRead => "DB_READ",
            Self::DbWrite => "DB_WRITE",
            Self::CacheRead => "CACHE_READ",
            Self::CacheWrite => "CACHE_WRITE",
            Self::OutboundHttp => "OUTBOUND_HTTP",
            Self::UploadRead => "UPLOAD_READ",
            Self::StorageRead => "STORAGE_READ",
            Self::StorageWrite => "STORAGE_WRITE",
            Self::AuthRead => "AUTH_READ",
            Self::SessionWrite => "SESSION_WRITE",
            Self::SecurityAudit => "SECURITY_AUDIT",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Effect {
    DbRead,
    DbWrite,
    SecurityAudit,
    Network(String),
}

impl Effect {
    pub const fn class(&self) -> EffectClass {
        match self {
            Self::DbRead => EffectClass::DbRead,
            Self::DbWrite => EffectClass::DbWrite,
            Self::SecurityAudit => EffectClass::SecurityAudit,
            Self::Network(_) => EffectClass::OutboundHttp,
        }
    }

    pub fn source_name(&self) -> String {
        match self {
            Self::DbRead => "db.read".into(),
            Self::DbWrite => "db.write".into(),
            Self::SecurityAudit => "security.audit".into(),
            Self::Network(target) => format!("net.{target}"),
        }
    }
}
