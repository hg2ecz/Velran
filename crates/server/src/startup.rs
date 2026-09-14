use crate::backend_support::build_hosting_runtime;
use crate::server_errors::StartupError;
use crate::startup_args::StartupArgs;
use crate::tls_support::build_tls_acceptor;
use observability::Metrics;
use std::sync::{Arc, RwLock};

pub(super) async fn run(parsed: StartupArgs) -> Result<(), StartupError> {
    let StartupArgs {
        app,
        config,
        db_config,
        auth: auth_cli,
        tls: tls_cli,
        web: mut web_cli,
        storage: storage_cli,
        resource_limits: _resource_limits,
        resource_profiles_file,
        static_assets: static_cli,
        lifecycle,
        rate_limits_file,
        allow_memory_rate_limit,
        observability: observability_cli,
        cache: cache_cli,
        native,
        log_config: _log_config,
        domains: domain_cli,
        unix_socket,
        behind_proxy,
        source_reload,
    } = parsed;
    if behind_proxy && unix_socket.is_some() {
        web_cli.trusted_proxy_cidrs.push("127.0.0.0/8".parse()?);
        web_cli.trusted_proxy_cidrs.push("::1/128".parse()?);
    }
    let production_db_config = db_config.clone();
    let metrics = Arc::new(Metrics::default());
    let hosting_value = build_hosting_runtime(
        &app,
        &config,
        &storage_cli,
        &static_cli,
        resource_profiles_file.as_deref(),
        &lifecycle,
        &domain_cli,
        &source_reload,
        &native,
    )?;
    let hosting = Arc::new(RwLock::new(hosting_value));
    let hosting_snapshot = hosting
        .read()
        .map_err(|_| StartupError::invalid("hosting runtime lock poisoned"))?
        .clone();
    crate::production_security::validate_static(
        &hosting_snapshot,
        &config,
        production_db_config.as_ref(),
        &web_cli,
        &source_reload,
        &native,
    )
    .map_err(|error| StartupError::invalid(error.to_string()))?;
    let prepared = crate::startup_services::prepare(crate::startup_services::ServicePreparation {
        hosting: &hosting,
        hosting_snapshot: &hosting_snapshot,
        db_config,
        auth: &auth_cli,
        config: &config,
        rate_limits_file: rate_limits_file.as_deref(),
        allow_memory_rate_limit,
        cache: &cache_cli,
        lifecycle: &lifecycle,
        web: &web_cli,
    })
    .await?;
    let crate::startup_services::PreparedServices {
        database,
        sessions,
        auth_runtime,
        route_rate_limiter,
        public_cache,
        idempotency_redis,
        outbound,
        source_reload_task,
    } = prepared;

    let multi_domain = !hosting_snapshot.domains.is_empty();
    let tls_acceptor = build_tls_acceptor(&tls_cli, &domain_cli)?;
    let reverse_proxy_https = crate::startup_security::validate_transport_configuration(
        multi_domain,
        &tls_cli,
        &config,
        &web_cli,
        unix_socket.as_deref(),
        behind_proxy,
        tls_acceptor.is_some(),
    )?;

    crate::production_security::validate_transport(
        &hosting_snapshot,
        tls_acceptor.is_some() || reverse_proxy_https,
    )
    .map_err(|error| StartupError::invalid(error.to_string()))?;

    crate::startup_transport::serve(crate::startup_transport::TransportRuntime {
        app,
        config,
        tls: tls_cli,
        web: web_cli,
        lifecycle,
        observability: observability_cli,
        cache: cache_cli,
        domains: domain_cli,
        unix_socket,
        behind_proxy,
        reverse_proxy_https,
        multi_domain,
        hosting,
        database,
        sessions,
        auth_runtime,
        route_rate_limiter,
        public_cache,
        idempotency_redis,
        outbound,
        metrics,
        source_reload_task,
        tls_acceptor,
    })
    .await
}
