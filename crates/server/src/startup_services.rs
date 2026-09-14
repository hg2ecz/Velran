use crate::AuthRuntime;
use crate::auth_setup::{build_ldap_config, load_roles, load_totp_secrets};
use crate::bootstrap_config::{PublicPageCache, load_rate_policies, validate_route_rate_policies};
use crate::membership_setup::load_memberships;
use crate::outbound_runtime::ServerOutbound;
use crate::rate_limit::RouteRateLimiter;
use crate::server_config_file::{DomainRuntime, HostingRuntime};
use crate::server_errors::StartupError;
use crate::source_reload::spawn_source_reload_supervisor;
use crate::{AuthCliConfig, CacheCliConfig, LifecycleCliConfig, WebSecurityCliConfig};
use auth::{
    AuthAbuseLimiter, AuthAbusePolicy, LocalUserStore, RedisSessionStore, SessionBackend,
    SessionStore, TotpReplayGuard,
};
use data::{Database, DbConfig, RedisConfig, RedisStore};
use language_core::ServerConfig;
use observability::server_log;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::task::JoinHandle;

pub(super) struct ServicePreparation<'a> {
    pub(super) hosting: &'a Arc<RwLock<HostingRuntime>>,
    pub(super) hosting_snapshot: &'a HostingRuntime,
    pub(super) db_config: Option<DbConfig>,
    pub(super) auth: &'a AuthCliConfig,
    pub(super) config: &'a ServerConfig,
    pub(super) rate_limits_file: Option<&'a Path>,
    pub(super) allow_memory_rate_limit: bool,
    pub(super) cache: &'a CacheCliConfig,
    pub(super) lifecycle: &'a LifecycleCliConfig,
    pub(super) web: &'a WebSecurityCliConfig,
}

pub(super) struct PreparedServices {
    pub(super) database: Option<Arc<Database>>,
    pub(super) sessions: SessionBackend,
    pub(super) auth_runtime: Arc<AuthRuntime>,
    pub(super) route_rate_limiter: Arc<RouteRateLimiter>,
    pub(super) public_cache: Arc<PublicPageCache>,
    pub(super) idempotency_redis: Option<RedisStore>,
    pub(super) outbound: Option<Arc<ServerOutbound>>,
    pub(super) source_reload_task: Option<JoinHandle<()>>,
}

pub(super) async fn prepare(
    input: ServicePreparation<'_>,
) -> Result<PreparedServices, StartupError> {
    let all_domains = unique_domains(input.hosting_snapshot);
    let database = connect_database(input.db_config, &all_domains).await?;
    let redis = connect_auth_redis(input.auth).await?;
    let route_rate_limiter = build_route_rate_limiter(
        input.rate_limits_file,
        &all_domains,
        redis.clone(),
        input.allow_memory_rate_limit,
    )?;
    let public_cache =
        build_public_cache(&all_domains, input.auth, redis.as_ref(), input.cache).await?;
    crate::startup_security::validate_replay_guard_requirements(
        &all_domains,
        redis.is_some(),
        input.web,
    )?;
    let sessions = build_sessions(redis.clone(), input.config);
    let idempotency_redis = redis.clone();
    let auth_runtime = build_auth_runtime(input.auth, redis).await?;
    crate::startup_security::validate_auth_requirements(&all_domains, &auth_runtime)?;
    let outbound = crate::outbound_startup::build(&all_domains, input.web)?;
    let source_reload_task = build_source_reload_task(
        input.hosting,
        &all_domains,
        input.lifecycle,
        &route_rate_limiter,
        &public_cache,
        input.cache,
        &auth_runtime,
        database.is_some(),
        idempotency_redis.is_some(),
        input.web.webhook_secrets_dir.is_some(),
        outbound.clone(),
    );

    Ok(PreparedServices {
        database,
        sessions,
        auth_runtime,
        route_rate_limiter,
        public_cache,
        idempotency_redis,
        outbound,
        source_reload_task,
    })
}

fn unique_domains(hosting: &HostingRuntime) -> Vec<Arc<DomainRuntime>> {
    let mut seen_domain_hosts = HashSet::new();
    hosting
        .default
        .iter()
        .cloned()
        .chain(hosting.domains.values().cloned())
        .filter(|domain| {
            domain
                .host
                .as_ref()
                .map(|host| seen_domain_hosts.insert(host.clone()))
                .unwrap_or(true)
        })
        .collect()
}

async fn connect_database(
    db_config: Option<DbConfig>,
    all_domains: &[Arc<DomainRuntime>],
) -> Result<Option<Arc<Database>>, StartupError> {
    let database = match db_config {
        Some(config) => {
            let database = Database::connect(config).await?;
            database.ping().await?;
            server_log(&format!("database: {:?} (connected)", database.backend()));
            Some(Arc::new(database))
        }
        None => None,
    };

    let database_required = all_domains.iter().any(|domain| {
        domain.program.pages.iter().any(|page| page.needs_db)
            || domain.program.actions.iter().any(|action| action.needs_db)
    });
    if database_required && database.is_none() {
        return Err(StartupError::invalid(
            "application declares `db: Db`, but server was started without --db-url",
        ));
    }
    Ok(database)
}

async fn connect_auth_redis(auth: &AuthCliConfig) -> Result<Option<RedisStore>, StartupError> {
    match auth.redis_url.clone() {
        Some(url) => {
            let mut config = RedisConfig::secure_default(url, "velran-auth");
            if auth.allow_insecure_redis {
                config.require_tls = false;
            }
            let store = RedisStore::connect(config).await?;
            store.ping().await?;
            server_log("auth redis: connected");
            Ok(Some(store))
        }
        None => Ok(None),
    }
}

fn build_route_rate_limiter(
    rate_limits_file: Option<&Path>,
    all_domains: &[Arc<DomainRuntime>],
    redis: Option<RedisStore>,
    allow_memory_rate_limit: bool,
) -> Result<Arc<RouteRateLimiter>, StartupError> {
    let policies = load_rate_policies(rate_limits_file)?;
    for domain in all_domains {
        validate_route_rate_policies(&domain.program, &policies)?;
    }
    if !policies.is_empty() && redis.is_none() && !allow_memory_rate_limit {
        return Err(StartupError::invalid(
            "application uses route rate policies but Redis is not configured; use Redis or explicit --allow-memory-rate-limit for development",
        ));
    }
    Ok(Arc::new(RouteRateLimiter {
        policies: Arc::new(policies),
        redis,
        memory: Arc::new(Mutex::new(HashMap::new())),
    }))
}

async fn build_public_cache(
    all_domains: &[Arc<DomainRuntime>],
    auth: &AuthCliConfig,
    redis: Option<&RedisStore>,
    cache: &CacheCliConfig,
) -> Result<Arc<PublicPageCache>, StartupError> {
    let mut cached_route_count = 0usize;
    for domain in all_domains {
        for route in domain
            .program
            .routes
            .iter()
            .filter(|route| route.public_cache.is_some())
        {
            cached_route_count += 1;
            if let Some(policy) = route.public_cache.as_ref() {
                let ttl = policy.ttl_secs;
                if ttl > cache.max_ttl_secs {
                    return Err(StartupError::invalid(format!(
                        "route `{}` cache ttl {} exceeds operator maximum {}",
                        route.name, ttl, cache.max_ttl_secs
                    )));
                }
            }
        }
    }
    if cached_route_count > 0 && redis.is_none() && !cache.allow_memory {
        return Err(StartupError::invalid(
            "application uses public cache but Redis is not configured; use Redis or explicit --allow-memory-cache for development",
        ));
    }

    let cache_redis = if cached_route_count > 0 {
        if let Some(url) = auth.redis_url.clone() {
            let mut config = RedisConfig::secure_default(url, "velran-cache");
            if auth.allow_insecure_redis {
                config.require_tls = false;
            }
            let store = RedisStore::connect(config).await?;
            store.ping().await?;
            Some(store)
        } else {
            None
        }
    } else {
        None
    };

    Ok(Arc::new(PublicPageCache::new(
        cache_redis,
        cache.max_entries,
        cache.max_bytes,
        cache.singleflight_wait_timeout_ms,
    )))
}

fn build_sessions(redis: Option<RedisStore>, config: &ServerConfig) -> SessionBackend {
    match redis {
        Some(store) => {
            SessionBackend::Redis(RedisSessionStore::new(store, config.session_ttl_secs))
        }
        None => SessionBackend::Memory(SessionStore::new(
            Duration::from_secs(config.session_ttl_secs),
            config.max_sessions,
        )),
    }
}

async fn build_auth_runtime(
    auth: &AuthCliConfig,
    redis: Option<RedisStore>,
) -> Result<Arc<AuthRuntime>, StartupError> {
    let ldap = build_ldap_config(auth)?;
    if ldap.is_some() && auth.local_auth_db_url.is_some() {
        return Err(StartupError::invalid(
            "configure exactly one primary auth backend: LDAP or --local-auth-db-url-file",
        ));
    }
    if auth.local_auth_db_url.is_some()
        && (auth.totp_secrets_file.is_some()
            || auth.roles_file.is_some()
            || auth.memberships_file.is_some())
    {
        return Err(StartupError::invalid(
            "--totp-secrets-file/--auth-roles-file/--auth-memberships-file belong to LDAP mode and cannot be combined with local auth",
        ));
    }
    let local = if let Some(url) = auth.local_auth_db_url.as_deref() {
        let store = LocalUserStore::connect_sqlite(url)
            .await
            .map_err(|_| StartupError::invalid("failed to connect local auth store"))?;
        store.ensure_ready().await.map_err(|_| {
            StartupError::invalid("local auth store is not initialized; run `velran-cli auth init`")
        })?;
        Some(store)
    } else {
        None
    };
    let totp_secrets = load_totp_secrets(auth.totp_secrets_file.as_deref())?;
    let roles = load_roles(auth.roles_file.as_deref())?;
    let memberships = load_memberships(auth.memberships_file.as_deref())?;
    let abuse_policy = AuthAbusePolicy {
        pair_max_attempts: auth.login_max_attempts,
        principal_max_attempts: auth.login_principal_max_attempts,
        source_max_attempts: auth.login_source_max_attempts,
        mfa_max_attempts: auth.mfa_max_attempts,
        window_secs: auth.login_window_secs,
    };
    let limiter = match redis.clone() {
        Some(store) => AuthAbuseLimiter::redis(store, abuse_policy),
        None => AuthAbuseLimiter::memory(abuse_policy),
    }
    .map_err(|_| StartupError::invalid("invalid authentication abuse policy"))?;

    Ok(Arc::new(AuthRuntime {
        ldap,
        local,
        totp_secrets,
        roles,
        memberships,
        require_totp: auth.require_totp,
        redis,
        local_totp: TotpReplayGuard::default(),
        limiter,
    }))
}

fn build_source_reload_task(
    hosting: &Arc<RwLock<HostingRuntime>>,
    all_domains: &[Arc<DomainRuntime>],
    lifecycle: &LifecycleCliConfig,
    route_rate_limiter: &Arc<RouteRateLimiter>,
    public_cache: &Arc<PublicPageCache>,
    cache: &CacheCliConfig,
    auth_runtime: &AuthRuntime,
    database_available: bool,
    idempotency_available: bool,
    webhook_secrets_available: bool,
    outbound: Option<Arc<ServerOutbound>>,
) -> Option<JoinHandle<()>> {
    if !all_domains.iter().any(|domain| domain.reload.enabled) {
        return None;
    }
    let auth_enabled = auth_runtime.ldap.is_some() || auth_runtime.local.is_some();
    let cache_available = public_cache.has_redis() || cache.allow_memory;
    server_log("{\"event\":\"source_reload_supervisor_started\"}");
    Some(spawn_source_reload_supervisor(
        Arc::clone(hosting),
        lifecycle.clone(),
        Arc::clone(route_rate_limiter),
        Arc::clone(public_cache),
        cache.max_ttl_secs,
        cache_available,
        auth_enabled,
        database_available,
        idempotency_available,
        webhook_secrets_available,
        outbound,
    ))
}
