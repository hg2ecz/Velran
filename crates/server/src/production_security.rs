use crate::WebSecurityCliConfig;
use crate::server_config_file::{HostingRuntime, ReloadMode, SourceReloadCliConfig};
use data::DbConfig;
use language_core::{ProductionPolicy, ServerConfig};
use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::sync::Arc;

#[derive(Debug)]
pub(super) enum ProductionPolicyError {
    HttpsRequired(String),
    HstsRequired(String),
    InsecureDevCookies(String),
    SourceReloadEnabled(String),
    DebugCompileErrorsEnabled(String),
    DebugRustcReproEnabled(String),
    MissingOriginAllowed(String),
    InsecureDatabaseTls(String),
    InsecureCorsOrigin(String),
}

impl fmt::Display for ProductionPolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HttpsRequired(label) => {
                write!(f, "production policy for {label} requires HTTPS")
            }
            Self::HstsRequired(label) => {
                write!(f, "production policy for {label} requires HSTS over HTTPS")
            }
            Self::InsecureDevCookies(label) => write!(
                f,
                "production policy for {label} forbids insecure development cookies"
            ),
            Self::SourceReloadEnabled(label) => write!(
                f,
                "production policy for {label} requires source reload to be disabled or use reload.mode=rolling"
            ),
            Self::DebugCompileErrorsEnabled(label) => write!(
                f,
                "production policy for {label} forbids detailed compiler diagnostics"
            ),
            Self::DebugRustcReproEnabled(label) => write!(
                f,
                "production policy for {label} forbids rustc reproduction artifacts"
            ),
            Self::MissingOriginAllowed(label) => write!(
                f,
                "production policy for {label} forbids missing browser Origin"
            ),
            Self::InsecureDatabaseTls(label) => write!(
                f,
                "production policy for {label} forbids disabling remote database TLS"
            ),
            Self::InsecureCorsOrigin(label) => write!(
                f,
                "production policy for {label} requires HTTPS CORS origins"
            ),
        }
    }
}

impl Error for ProductionPolicyError {}

pub(super) fn validate_static(
    hosting: &HostingRuntime,
    config: &ServerConfig,
    db: Option<&DbConfig>,
    web: &WebSecurityCliConfig,
    _source_reload: &SourceReloadCliConfig,
    native: &crate::server_config_file::NativeCliConfig,
) -> Result<(), ProductionPolicyError> {
    for domain in unique_domains(hosting) {
        let Some(policy) = domain.program.production else {
            continue;
        };
        validate_static_policy(
            policy,
            &domain_label(domain.host.as_deref()),
            config,
            db,
            web,
            domain.reload.enabled && domain.reload.mode != ReloadMode::Rolling,
            domain.reload.debug_compile_errors,
            native.debug_rustc_repro,
        )?;
    }
    Ok(())
}

pub(super) fn validate_transport(
    hosting: &HostingRuntime,
    effective_https: bool,
) -> Result<(), ProductionPolicyError> {
    for domain in unique_domains(hosting) {
        let Some(policy) = domain.program.production else {
            continue;
        };
        let label = domain_label(domain.host.as_deref());
        if policy.https_required && !effective_https {
            return Err(ProductionPolicyError::HttpsRequired(label));
        }
        if policy.hsts_required && !effective_https {
            return Err(ProductionPolicyError::HstsRequired(label));
        }
    }
    Ok(())
}

fn validate_static_policy(
    policy: ProductionPolicy,
    label: &str,
    config: &ServerConfig,
    db: Option<&DbConfig>,
    web: &WebSecurityCliConfig,
    unsafe_reload_enabled: bool,
    debug_compile_errors: bool,
    debug_rustc_repro: bool,
) -> Result<(), ProductionPolicyError> {
    if policy.debug_disabled {
        if config.insecure_dev_cookies {
            return Err(ProductionPolicyError::InsecureDevCookies(label.into()));
        }
        if unsafe_reload_enabled {
            return Err(ProductionPolicyError::SourceReloadEnabled(label.into()));
        }
        if debug_compile_errors {
            return Err(ProductionPolicyError::DebugCompileErrorsEnabled(
                label.into(),
            ));
        }
        if debug_rustc_repro {
            return Err(ProductionPolicyError::DebugRustcReproEnabled(label.into()));
        }
        if web.allow_missing_origin {
            return Err(ProductionPolicyError::MissingOriginAllowed(label.into()));
        }
    }
    if policy.database_tls_required && db.map_or(false, |value| !value.require_tls_for_remote) {
        return Err(ProductionPolicyError::InsecureDatabaseTls(label.into()));
    }
    if web
        .cors_origins
        .iter()
        .any(|origin| !origin.starts_with("https://"))
    {
        return Err(ProductionPolicyError::InsecureCorsOrigin(label.into()));
    }
    Ok(())
}

fn unique_domains(hosting: &HostingRuntime) -> Vec<Arc<crate::server_config_file::DomainRuntime>> {
    let mut seen = HashSet::new();
    hosting
        .default
        .iter()
        .cloned()
        .chain(hosting.domains.values().cloned())
        .filter(|domain| {
            domain
                .host
                .as_ref()
                .map(|host| seen.insert(host.clone()))
                .unwrap_or(true)
        })
        .collect()
}

fn domain_label(host: Option<&str>) -> String {
    host.map(|value| format!("domain `{value}`"))
        .unwrap_or_else(|| "the default application".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_production_accepts_rolling_reload_contract_but_not_development_reload() {
        let config = ServerConfig::default();
        let web = WebSecurityCliConfig::default();
        assert!(
            validate_static_policy(
                ProductionPolicy::STRICT,
                "test",
                &config,
                None,
                &web,
                false,
                false,
                false,
            )
            .is_ok()
        );
        assert!(matches!(
            validate_static_policy(
                ProductionPolicy::STRICT,
                "test",
                &config,
                None,
                &web,
                true,
                false,
                false,
            ),
            Err(ProductionPolicyError::SourceReloadEnabled(_))
        ));
    }
}
