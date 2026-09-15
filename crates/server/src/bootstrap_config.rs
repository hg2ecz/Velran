use crate::rate_limit::{RatePolicy, RateScope};
use crate::resource_limits::ResourceLimitConfig;
use crate::server_config_file::SourceReloadCliConfig;
use crate::server_errors::{
    CliValueError, PublicCacheError, RatePolicyConfigError, ResourceProfileConfigError,
    SecretFileError,
};
use crate::{
    AuthCliConfig, CacheCliConfig, LifecycleCliConfig, ObservabilityCliConfig,
    StaticAssetsCliConfig, StorageCliConfig, TlsCliConfig, WebSecurityCliConfig, unix_secs,
};
use data::RedisStore;
use language_core::{RouteAuth, ServerConfig};
use observability::LogConfig;
use runtime::{ExecutionLimits, ResourceProfiles};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub(super) fn print_effective_config(
    app: &Path,
    config: &ServerConfig,
    tls: &TlsCliConfig,
    web: &WebSecurityCliConfig,
    storage: &StorageCliConfig,
    static_assets: &StaticAssetsCliConfig,
    lifecycle: &LifecycleCliConfig,
    observability: &ObservabilityCliConfig,
    logging: &LogConfig,
    cache: &CacheCliConfig,
    source_reload: &SourceReloadCliConfig,
    resource_limits: &ResourceLimitConfig,
    resource_profiles_file: Option<&Path>,
    rate_limits_file: Option<&Path>,
    auth: &AuthCliConfig,
    db_configured: bool,
) {
    println!("# effective Velran server config (secrets redacted)");
    println!(
        "[server]\napp = {:?}\nlisten = {:?}",
        app.display().to_string(),
        config.listen.to_string()
    );
    println!(
        "[tls]\nconfigured = {}\npublic_host = {:?}\nhandshake_timeout_ms = {}",
        tls.cert_file.is_some(),
        tls.public_host,
        tls.handshake_timeout_ms
    );
    println!(
        "[reload]\nenabled = {}\nmode = \"{}\"\npoll_interval_ms = {}\ndebounce_ms = {}\ndebug_compile_errors = {}",
        source_reload.enabled,
        source_reload.mode.as_str(),
        source_reload.poll_interval_ms,
        source_reload.debounce_ms,
        source_reload.debug_compile_errors
    );
    println!("[database]\nconfigured = {}", db_configured);
    println!("[redis]\nconfigured = {}", auth.redis_url.is_some());
    println!(
        "[auth]\nldap_configured = {}\nlocal_auth_configured = {}\nrequire_totp = {}",
        auth.ldap_url.is_some(),
        auth.local_auth_db_url.is_some(),
        auth.require_totp
    );
    println!(
        "[web]\ntrusted_proxy_count = {}\ncors_origin_count = {}\ncors_allow_credentials = {}\nwebhook_secrets_dir = {:?}\negress_policy_file = {:?}",
        web.trusted_proxy_cidrs.len(),
        web.cors_origins.len(),
        web.cors_allow_credentials,
        web.webhook_secrets_dir
            .as_ref()
            .map(|p| p.display().to_string()),
        web.egress_policy_file
            .as_ref()
            .map(|p| p.display().to_string())
    );
    println!(
        "[storage]\ndata_root = {:?}\nfs_mode = {:?}\nmax_upload_bytes = {}\nmax_image_pixels = {}",
        storage.data_root.as_ref().map(|p| p.display().to_string()),
        storage.fs_mode,
        storage.max_upload_bytes,
        storage.max_image_pixels
    );
    println!(
        "[static_assets]\nroot = {:?}\nurl_prefix = {:?}\nprecompressed = {}",
        static_assets.root.as_ref().map(|p| p.display().to_string()),
        static_assets.url_prefix,
        static_assets.precompressed
    );
    println!(
        "[lifecycle]\nhealth_live_path = {:?}\nhealth_ready_path = {:?}\nshutdown_grace_ms = {}",
        lifecycle.live_path, lifecycle.ready_path, lifecycle.shutdown_grace_ms
    );
    println!(
        "[observability]\nmetrics_listen = {:?}\naccess_log = {}",
        observability.metrics_listen.map(|v| v.to_string()),
        observability.access_log
    );
    println!(
        "[logging]\nserver_file = {:?}\naccess_file = {:?}\naudit_file = {:?}\nstderr = {}",
        logging
            .server_file
            .as_ref()
            .map(|p| p.display().to_string()),
        logging
            .access_file
            .as_ref()
            .map(|p| p.display().to_string()),
        logging.audit_file.as_ref().map(|p| p.display().to_string()),
        logging.stderr
    );
    println!(
        "[cache]\nmax_ttl_secs = {}\nmax_entries = {}\nmax_bytes = {}\nallow_memory = {}",
        cache.max_ttl_secs, cache.max_entries, cache.max_bytes, cache.allow_memory
    );
    println!(
        "[limits]\nmax_connections = {}\nmax_instructions = {}\nmax_runtime_alloc_bytes = {}\nresource_profiles_file = {:?}",
        config.max_connections,
        config.max_instructions,
        config.max_runtime_alloc_bytes,
        resource_profiles_file.map(|p| p.display().to_string())
    );
    println!(
        "[rate_limit]\npolicies_file = {:?}",
        rate_limits_file.map(|p| p.display().to_string())
    );
    println!(
        "[process]\nmax_address_space_bytes = {:?}\ncgroup_dir = {:?}",
        resource_limits.max_address_space_bytes,
        resource_limits
            .cgroup_dir
            .as_ref()
            .map(|p| p.display().to_string())
    );
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct CachedPage {
    pub(super) content_type: String,
    pub(super) body: Vec<u8>,
}
#[derive(Clone)]
struct MemoryCacheEntry {
    expires_at: u64,
    inserted_at: u64,
    value: CachedPage,
    bytes: usize,
}
#[derive(Clone)]
pub(super) struct PublicPageCache {
    redis: Option<RedisStore>,
    memory: Arc<Mutex<HashMap<String, MemoryCacheEntry>>>,
    generations: Arc<Mutex<HashMap<String, u64>>>,
    rebuild_locks: Arc<Mutex<HashMap<String, Arc<tokio::sync::Semaphore>>>>,
    max_entries: usize,
    max_bytes: usize,
    singleflight_wait_timeout_ms: u64,
}
impl PublicPageCache {
    pub(super) fn new(
        redis: Option<RedisStore>,
        max_entries: usize,
        max_bytes: usize,
        singleflight_wait_timeout_ms: u64,
    ) -> Self {
        Self {
            redis,
            memory: Arc::new(Mutex::new(HashMap::new())),
            generations: Arc::new(Mutex::new(HashMap::new())),
            rebuild_locks: Arc::new(Mutex::new(HashMap::new())),
            max_entries,
            max_bytes,
            singleflight_wait_timeout_ms,
        }
    }

    pub(super) fn has_redis(&self) -> bool {
        self.redis.is_some()
    }

    pub(super) fn singleflight_wait_timeout_ms(&self) -> u64 {
        self.singleflight_wait_timeout_ms
    }
    pub(super) fn prune_rebuild_locks(&self) -> Result<(), PublicCacheError> {
        let mut locks = self
            .rebuild_locks
            .lock()
            .map_err(|_| PublicCacheError::LockPoisoned("rebuild"))?;
        if locks.len() >= 10_000 {
            locks.retain(|_, lock| Arc::strong_count(lock) > 1 || lock.available_permits() == 0);
        }
        Ok(())
    }
    pub(super) fn rebuild_lock(
        &self,
        key: &str,
    ) -> Result<Arc<tokio::sync::Semaphore>, PublicCacheError> {
        let mut locks = self
            .rebuild_locks
            .lock()
            .map_err(|_| PublicCacheError::LockPoisoned("rebuild"))?;
        Ok(Arc::clone(locks.entry(key.into()).or_insert_with(|| {
            Arc::new(tokio::sync::Semaphore::new(1))
        })))
    }
    pub(super) async fn generation(&self, route: &str) -> Result<u64, PublicCacheError> {
        if let Some(redis) = &self.redis {
            let key = format!("generation:{route}");
            let raw = redis.get(&key).await?;
            return match raw {
                Some(v) => String::from_utf8(v)
                    .map_err(PublicCacheError::GenerationUtf8)?
                    .parse()
                    .map_err(PublicCacheError::GenerationNumber),
                None => Ok(0),
            };
        }
        Ok(*self
            .generations
            .lock()
            .map_err(|_| PublicCacheError::LockPoisoned("generation"))?
            .get(route)
            .unwrap_or(&0))
    }
    pub(super) async fn invalidate_route(&self, route: &str) -> Result<(), PublicCacheError> {
        if let Some(redis) = &self.redis {
            redis.increment(&format!("generation:{route}"), 1).await?;
            return Ok(());
        }
        let mut g = self
            .generations
            .lock()
            .map_err(|_| PublicCacheError::LockPoisoned("generation"))?;
        let v = g.entry(route.into()).or_insert(0);
        *v = v.saturating_add(1);
        Ok(())
    }
    pub(super) async fn get(&self, key: &str) -> Result<Option<CachedPage>, PublicCacheError> {
        if let Some(redis) = &self.redis {
            let raw = redis.get(&format!("page:{key}")).await?;
            return raw
                .map(|v| serde_json::from_slice(&v).map_err(PublicCacheError::from))
                .transpose();
        }
        let now = unix_secs()?;
        let mut map = self
            .memory
            .lock()
            .map_err(|_| PublicCacheError::LockPoisoned("memory"))?;
        if map.get(key).is_some_and(|e| e.expires_at <= now) {
            map.remove(key);
        }
        Ok(map.get(key).map(|e| e.value.clone()))
    }
    pub(super) async fn set(
        &self,
        key: &str,
        value: CachedPage,
        ttl: u64,
    ) -> Result<(), PublicCacheError> {
        if let Some(redis) = &self.redis {
            let raw = serde_json::to_vec(&value)?;
            return redis
                .set(&format!("page:{key}"), &raw, Some(ttl))
                .await
                .map_err(PublicCacheError::from);
        }
        let bytes = value.body.len().saturating_add(value.content_type.len());
        if bytes > self.max_bytes {
            return Ok(());
        }
        let now = unix_secs()?;
        let mut map = self
            .memory
            .lock()
            .map_err(|_| PublicCacheError::LockPoisoned("memory"))?;
        map.retain(|_, e| e.expires_at > now);
        let mut total: usize = map.values().map(|e| e.bytes).sum();
        while (map.len() >= self.max_entries || total.saturating_add(bytes) > self.max_bytes)
            && !map.is_empty()
        {
            let victim = map
                .iter()
                .min_by_key(|(_, e)| e.inserted_at)
                .map(|(k, _)| k.clone())
                .unwrap();
            if let Some(v) = map.remove(&victim) {
                total = total.saturating_sub(v.bytes);
            }
        }
        map.insert(
            key.into(),
            MemoryCacheEntry {
                expires_at: now.saturating_add(ttl),
                inserted_at: now,
                value,
                bytes,
            },
        );
        Ok(())
    }
}
pub(super) fn load_rate_policies(
    path: Option<&Path>,
) -> Result<HashMap<String, RatePolicy>, RatePolicyConfigError> {
    let Some(path) = path else {
        return Ok(HashMap::new());
    };
    let text = fs::read_to_string(path).map_err(|source| RatePolicyConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let mut section = None::<String>;
    let mut raw: HashMap<String, HashMap<String, String>> = HashMap::new();
    for (idx, line0) in text.lines().enumerate() {
        let line_no = idx + 1;
        let line = line0.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            let section_name = line[1..line.len() - 1].trim();
            let name = section_name.strip_prefix("policy.").ok_or_else(|| {
                RatePolicyConfigError::Syntax {
                    path: path.to_path_buf(),
                    line: line_no,
                    message: "expected [policy.NAME]".into(),
                }
            })?;
            if name.is_empty() || !name.bytes().all(|b| b == b'_' || b.is_ascii_alphanumeric()) {
                return Err(RatePolicyConfigError::Syntax {
                    path: path.to_path_buf(),
                    line: line_no,
                    message: "invalid policy name".into(),
                });
            }
            if raw.contains_key(name) {
                return Err(RatePolicyConfigError::Syntax {
                    path: path.to_path_buf(),
                    line: line_no,
                    message: format!("duplicate policy section `{name}`"),
                });
            }
            raw.insert(name.into(), HashMap::new());
            section = Some(name.into());
            continue;
        }
        let name = section
            .clone()
            .ok_or_else(|| RatePolicyConfigError::Syntax {
                path: path.to_path_buf(),
                line: line_no,
                message: "key outside policy section".into(),
            })?;
        let (k, v) = line
            .split_once('=')
            .ok_or_else(|| RatePolicyConfigError::Syntax {
                path: path.to_path_buf(),
                line: line_no,
                message: "expected key = value".into(),
            })?;
        let key = k.trim().to_string();
        let map = raw.get_mut(&name).expect("section inserted");
        if map.contains_key(&key) {
            return Err(RatePolicyConfigError::Syntax {
                path: path.to_path_buf(),
                line: line_no,
                message: format!("duplicate key `{key}` in policy `{name}`"),
            });
        }
        map.insert(key, v.trim().trim_matches('"').into());
    }

    let mut out = HashMap::new();
    for (name, values) in raw {
        let limit_raw = values
            .get("limit")
            .ok_or_else(|| RatePolicyConfigError::MissingField {
                policy: name.clone(),
                field: "limit",
            })?;
        let limit = limit_raw
            .parse()
            .map_err(|source| RatePolicyConfigError::InvalidNumber {
                policy: name.clone(),
                field: "limit",
                source,
            })?;
        let window_raw =
            values
                .get("window_secs")
                .ok_or_else(|| RatePolicyConfigError::MissingField {
                    policy: name.clone(),
                    field: "window_secs",
                })?;
        let window_secs =
            window_raw
                .parse()
                .map_err(|source| RatePolicyConfigError::InvalidNumber {
                    policy: name.clone(),
                    field: "window_secs",
                    source,
                })?;
        if limit == 0 || window_secs == 0 || window_secs > 604799 {
            return Err(RatePolicyConfigError::InvalidLimits { policy: name });
        }
        let scope = match values
            .get("scope")
            .map(String::as_str)
            .unwrap_or("ip_route")
        {
            "ip" => RateScope::Ip,
            "route" => RateScope::Route,
            "ip_route" => RateScope::IpRoute,
            "user" => RateScope::User,
            "user_route" => RateScope::UserRoute,
            other => {
                return Err(RatePolicyConfigError::UnknownScope {
                    policy: name,
                    scope: other.into(),
                });
            }
        };
        for key in values.keys() {
            if !matches!(key.as_str(), "limit" | "window_secs" | "scope") {
                return Err(RatePolicyConfigError::UnknownKey {
                    policy: name,
                    key: key.clone(),
                });
            }
        }
        out.insert(
            name,
            RatePolicy {
                limit,
                window_secs,
                scope,
            },
        );
    }
    Ok(out)
}

pub(super) fn validate_route_rate_policies(
    program: &language_core::Program,
    policies: &HashMap<String, RatePolicy>,
) -> Result<(), RatePolicyConfigError> {
    for route in &program.routes {
        if let Some(name) = route.rate_policy.as_deref() {
            let policy =
                policies
                    .get(name)
                    .ok_or_else(|| RatePolicyConfigError::UnknownRoutePolicy {
                        route: route.name.clone(),
                        policy: name.to_string(),
                    })?;
            if matches!(policy.scope, RateScope::User | RateScope::UserRoute)
                && matches!(route.auth, RouteAuth::Public | RouteAuth::Webhook(_))
            {
                return Err(RatePolicyConfigError::PublicUserScopedPolicy {
                    route: route.name.clone(),
                    policy: name.to_string(),
                });
            }
        }
    }
    Ok(())
}

pub(super) fn load_resource_profiles(
    path: Option<&Path>,
    request: &ExecutionLimits,
    default_concurrency: usize,
) -> Result<ResourceProfiles, ResourceProfileConfigError> {
    crate::resource_profile_bootstrap::load_resource_profiles(path, request, default_concurrency)
}

pub(super) fn audit_resource_profiles(
    program: &language_core::Program,
    profiles: &ResourceProfiles,
) -> Result<(), ResourceProfileConfigError> {
    crate::resource_profile_bootstrap::audit_resource_profiles(program, profiles)
}

pub(super) fn json_log_escape(v: &str) -> String {
    let mut out = String::new();
    for c in v.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => out.push('?'),
            c => out.push(c),
        }
    }
    out
}

pub(super) fn read_secret_file(path: &str) -> Result<String, SecretFileError> {
    let path_buf = PathBuf::from(path);
    let v = fs::read_to_string(&path_buf)
        .map_err(|source| SecretFileError::Read {
            path: path_buf.clone(),
            source,
        })?
        .trim()
        .to_string();
    if v.is_empty() {
        return Err(SecretFileError::Empty { path: path_buf });
    }
    Ok(v)
}
pub(super) fn parse_usize(
    args: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<usize, CliValueError> {
    let raw = args.next().ok_or_else(|| CliValueError::MissingValue {
        flag: flag.to_string(),
    })?;
    raw.parse().map_err(|source| CliValueError::InvalidNumber {
        flag: flag.to_string(),
        source,
    })
}
pub(super) fn parse_nonzero(
    args: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<usize, CliValueError> {
    let value = parse_usize(args, flag)?;
    if value == 0 {
        return Err(CliValueError::MustBePositive {
            flag: flag.to_string(),
        });
    }
    Ok(value)
}
pub(super) fn parse_u64(
    args: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<u64, CliValueError> {
    let raw = args.next().ok_or_else(|| CliValueError::MissingValue {
        flag: flag.to_string(),
    })?;
    raw.parse().map_err(|source| CliValueError::InvalidNumber {
        flag: flag.to_string(),
        source,
    })
}
