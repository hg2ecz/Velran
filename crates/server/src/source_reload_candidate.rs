use crate::LifecycleCliConfig;
use crate::backend_support::prepare_domain_runtime;
use crate::bootstrap_config::validate_route_rate_policies;
use crate::outbound_runtime::ServerOutbound;
use crate::rate_limit::RouteRateLimiter;
use crate::server_config_file::DomainRuntime;
use crate::server_errors::SourceReloadError;
use language_core::RouteAuth;
use std::sync::Arc;

fn validate_candidate(
    domain: &DomainRuntime,
    route_rate_limiter: &RouteRateLimiter,
    cache_max_ttl_secs: u64,
    cache_available: bool,
    auth_enabled: bool,
    database_available: bool,
    idempotency_available: bool,
    webhook_secrets_available: bool,
    outbound: Option<&ServerOutbound>,
) -> Result<(), SourceReloadError> {
    validate_route_rate_policies(&domain.program, route_rate_limiter.policies.as_ref())?;
    for route in &domain.program.routes {
        if let Some(public_cache) = route.public_cache.as_ref() {
            if public_cache.ttl_secs > cache_max_ttl_secs {
                return Err(SourceReloadError::CacheTtlExceeded {
                    domain: domain.host.clone(),
                    route: route.name.clone(),
                });
            }
        }
    }
    if domain
        .program
        .routes
        .iter()
        .any(|route| route.public_cache.is_some())
        && !cache_available
    {
        return Err(SourceReloadError::CacheUnavailable);
    }
    if (domain.program.pages.iter().any(|page| page.needs_db)
        || domain.program.actions.iter().any(|action| action.needs_db))
        && !database_available
    {
        return Err(SourceReloadError::DatabaseUnavailable);
    }
    if domain
        .program
        .routes
        .iter()
        .any(|route| route.idempotent || matches!(route.auth, RouteAuth::Webhook(_)))
        && !idempotency_available
    {
        return Err(SourceReloadError::IdempotencyUnavailable);
    }
    if domain
        .program
        .routes
        .iter()
        .any(|route| matches!(route.auth, RouteAuth::Webhook(_)))
        && !webhook_secrets_available
    {
        return Err(SourceReloadError::WebhookSecretsUnavailable);
    }
    crate::reload_security::validate_outbound(&domain.program, outbound)?;
    if domain
        .program
        .routes
        .iter()
        .any(|route| !matches!(route.auth, RouteAuth::Public | RouteAuth::Webhook(_)))
        && !auth_enabled
    {
        return Err(SourceReloadError::AuthenticationUnavailable);
    }
    Ok(())
}

pub(super) fn build_candidate(
    current: &Arc<DomainRuntime>,
    lifecycle: &LifecycleCliConfig,
    route_rate_limiter: &RouteRateLimiter,
    cache_max_ttl_secs: u64,
    cache_available: bool,
    auth_enabled: bool,
    database_available: bool,
    idempotency_available: bool,
    webhook_secrets_available: bool,
    outbound: Option<&ServerOutbound>,
) -> Result<Arc<DomainRuntime>, SourceReloadError> {
    let candidate = prepare_domain_runtime(
        current.host.clone(),
        current.workdir.clone(),
        &current.app,
        current.config.clone(),
        current.storage_cli.clone(),
        current.static_cli.clone(),
        current.resource_profiles_file.as_deref(),
        current.max_concurrent_requests,
        current.max_queued_requests,
        current.queue_timeout_ms,
        lifecycle,
        current.reload.clone(),
        &current.native_config,
        current.generation.saturating_add(1),
    )?;
    if candidate.program.production != current.program.production {
        return Err(SourceReloadError::ProductionPolicyChanged);
    }
    validate_candidate(
        &candidate,
        route_rate_limiter,
        cache_max_ttl_secs,
        cache_available,
        auth_enabled,
        database_available,
        idempotency_available,
        webhook_secrets_available,
        outbound,
    )?;
    Ok(candidate)
}
