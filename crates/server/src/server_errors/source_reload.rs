use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub(crate) enum SourceReloadError {
    Backend(super::BackendSupportError),
    RatePolicy(super::RatePolicyConfigError),
    Cache(super::PublicCacheError),
    HostingLockPoisoned,
    CacheTtlExceeded {
        domain: Option<String>,
        route: String,
    },
    CacheUnavailable,
    DatabaseUnavailable,
    AuthenticationUnavailable,
    IdempotencyUnavailable,
    WebhookSecretsUnavailable,
    OutboundUnavailable,
    OutboundTargetUnavailable(String),
    ProductionPolicyChanged,
}

impl SourceReloadError {
    pub(crate) fn compiler_error(&self) -> Option<&compiler::CompileError> {
        match self {
            Self::Backend(source) => source.compiler_error(),
            _ => None,
        }
    }

    pub(crate) fn rustc_diagnostics(&self) -> Option<&str> {
        match self {
            Self::Backend(source) => source.rustc_diagnostics(),
            _ => None,
        }
    }
}

impl fmt::Display for SourceReloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(source) => write!(f, "reload candidate preparation failed: {source}"),
            Self::RatePolicy(source) => write!(f, "reload rate policy validation failed: {source}"),
            Self::Cache(source) => write!(f, "reload cache invalidation failed: {source}"),
            Self::HostingLockPoisoned => write!(f, "hosting runtime lock poisoned"),
            Self::CacheTtlExceeded { domain, route } => write!(
                f,
                "domain {domain:?} route `{route}` cache ttl exceeds configured operator maximum"
            ),
            Self::CacheUnavailable => write!(
                f,
                "reloaded application uses public cache but no Redis cache or explicit memory cache is available"
            ),
            Self::DatabaseUnavailable => write!(
                f,
                "reloaded application declares `db: Db`, but the server has no database connection"
            ),
            Self::AuthenticationUnavailable => write!(
                f,
                "reloaded application requires authentication but no authentication backend is active"
            ),
            Self::IdempotencyUnavailable => write!(
                f,
                "reloaded application uses idempotent/webhook replay protection but Redis is unavailable"
            ),
            Self::WebhookSecretsUnavailable => write!(
                f,
                "reloaded application declares verified webhook routes but web.webhook_secrets_dir is unavailable"
            ),
            Self::OutboundUnavailable => write!(
                f,
                "reloaded application declares outbound integration effects but no egress policy client is active"
            ),
            Self::OutboundTargetUnavailable(target) => write!(
                f,
                "reloaded application requires outbound target `{target}` that is unavailable or ambiguous in the active egress policy"
            ),
            Self::ProductionPolicyChanged => write!(
                f,
                "reloaded application changed its production deployment policy; restart with an explicitly reviewed server configuration"
            ),
        }
    }
}

impl Error for SourceReloadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Backend(source) => Some(source),
            Self::RatePolicy(source) => Some(source),
            Self::Cache(source) => Some(source),
            _ => None,
        }
    }
}

impl From<super::BackendSupportError> for SourceReloadError {
    fn from(value: super::BackendSupportError) -> Self {
        Self::Backend(value)
    }
}
impl From<super::RatePolicyConfigError> for SourceReloadError {
    fn from(value: super::RatePolicyConfigError) -> Self {
        Self::RatePolicy(value)
    }
}
impl From<super::PublicCacheError> for SourceReloadError {
    fn from(value: super::PublicCacheError) -> Self {
        Self::Cache(value)
    }
}
