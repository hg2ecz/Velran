use crate::server_errors::ServerConfigError;
use crate::source_reload::SourceFileState;
use crate::static_delivery::validate_static_prefix;
use crate::tls_support::validate_public_host;
use crate::{StaticAssets, StaticAssetsCliConfig, StorageCliConfig};
use language_core::ServerConfig;
use observability::LogConfig;
use runtime::ResourceProfiles;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use storage::AppFs;
use tokio::sync::Semaphore;

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct ServerFileConfig {
    pub(super) server: FileServer,
    pub(super) tls: FileTls,
    pub(super) database: FileDatabase,
    pub(super) redis: FileRedis,
    pub(super) auth: FileAuth,
    pub(super) web: FileWeb,
    pub(super) storage: FileStorage,
    pub(super) static_assets: FileStatic,
    pub(super) lifecycle: FileLifecycle,
    pub(super) observability: FileObservability,
    pub(super) logging: FileLogging,
    pub(super) rate_limit: FileRateLimit,
    pub(super) cache: FileCache,
    pub(super) native: FileNative,
    pub(super) reload: FileReload,
    pub(super) limits: FileLimits,
    pub(super) cgroup: FileCgroup,
    pub(super) domains: Vec<FileDomain>,
}
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct FileServer {
    pub(super) app: Option<String>,
    pub(super) listen: Option<String>,
    pub(super) insecure_dev_cookies: Option<bool>,
    pub(super) unix_socket: Option<String>,
    pub(super) behind_proxy: Option<bool>,
    pub(super) expected_build_id: Option<String>,
}
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct FileTls {
    pub(super) cert_file: Option<String>,
    pub(super) key_file: Option<String>,
    pub(super) handshake_timeout_ms: Option<u64>,
    pub(super) http_redirect_listen: Option<String>,
    pub(super) public_host: Option<String>,
}
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct FileDatabase {
    pub(super) url_file: Option<String>,
    pub(super) allow_insecure: Option<bool>,
}
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct FileRedis {
    pub(super) url_file: Option<String>,
    pub(super) allow_insecure: Option<bool>,
}
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct FileAuth {
    pub(super) ldap_url: Option<String>,
    pub(super) ldap_search_base: Option<String>,
    pub(super) ldap_username_attribute: Option<String>,
    pub(super) ldap_service_bind_dn_file: Option<String>,
    pub(super) ldap_service_bind_password_file: Option<String>,
    pub(super) totp_secrets_file: Option<String>,
    pub(super) roles_file: Option<String>,
    pub(super) memberships_file: Option<String>,
    pub(super) local_auth_db_url_file: Option<String>,
    pub(super) require_totp: Option<bool>,
    pub(super) login_max_attempts: Option<u32>,
    pub(super) login_principal_max_attempts: Option<u32>,
    pub(super) login_source_max_attempts: Option<u32>,
    pub(super) mfa_max_attempts: Option<u32>,
    pub(super) login_window_secs: Option<u64>,
}
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct FileWeb {
    pub(super) trusted_proxy_cidrs: Option<Vec<String>>,
    pub(super) allow_missing_origin: Option<bool>,
    pub(super) cors_origins: Option<Vec<String>>,
    pub(super) cors_allow_credentials: Option<bool>,
    pub(super) webhook_secrets_dir: Option<String>,
    pub(super) egress_policy_file: Option<String>,
}
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct FileStorage {
    pub(super) data_root: Option<String>,
    pub(super) fs_mode: Option<String>,
    pub(super) max_upload_bytes: Option<u64>,
    pub(super) max_image_pixels: Option<u64>,
}
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct FileStatic {
    pub(super) root: Option<String>,
    pub(super) url_prefix: Option<String>,
    pub(super) max_asset_bytes: Option<u64>,
    pub(super) max_age_secs: Option<u64>,
    pub(super) immutable_max_age_secs: Option<u64>,
    pub(super) precompressed: Option<bool>,
}
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct FileLifecycle {
    pub(super) health_live_path: Option<String>,
    pub(super) health_ready_path: Option<String>,
    pub(super) health_dependency_timeout_ms: Option<u64>,
    pub(super) shutdown_grace_ms: Option<u64>,
}
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct FileObservability {
    pub(super) metrics_listen: Option<String>,
    pub(super) allow_public_metrics: Option<bool>,
    pub(super) access_log: Option<bool>,
}
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct FileLogging {
    pub(super) server_file: Option<String>,
    pub(super) access_file: Option<String>,
    pub(super) audit_file: Option<String>,
    pub(super) stderr: Option<bool>,
}
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct FileRateLimit {
    pub(super) policies_file: Option<String>,
    pub(super) allow_memory: Option<bool>,
}
#[derive(Debug, Clone)]
pub(super) struct NativeCliConfig {
    pub(super) enabled: bool,
    pub(super) rustc: Option<PathBuf>,
    pub(super) cache_dir: Option<PathBuf>,
    pub(super) optimization: NativeOptimizationCli,
    pub(super) target_cpu: Option<String>,
    pub(super) cpu_features: Option<String>,
    pub(super) required: bool,
    pub(super) debug_rustc_repro: bool,
}

impl Default for NativeCliConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rustc: None,
            cache_dir: None,
            optimization: NativeOptimizationCli::Release,
            target_cpu: None,
            cpu_features: None,
            required: false,
            debug_rustc_repro: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NativeOptimizationCli {
    Interactive,
    Release,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct FileNative {
    pub(super) enabled: Option<bool>,
    pub(super) rustc: Option<String>,
    pub(super) cache_dir: Option<String>,
    pub(super) optimization: Option<String>,
    pub(super) target_cpu: Option<String>,
    pub(super) cpu_features: Option<String>,
    pub(super) required: Option<bool>,
    pub(super) debug_rustc_repro: Option<bool>,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct FileCache {
    pub(super) max_ttl_secs: Option<u64>,
    pub(super) max_entries: Option<usize>,
    pub(super) max_bytes: Option<usize>,
    pub(super) allow_memory: Option<bool>,
    pub(super) singleflight_wait_timeout_ms: Option<u64>,
}
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct FileReload {
    pub(super) enabled: Option<bool>,
    pub(super) poll_interval_ms: Option<u64>,
    pub(super) debounce_ms: Option<u64>,
    pub(super) debug_compile_errors: Option<bool>,
}
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct FileLimits {
    pub(super) max_header_bytes: Option<usize>,
    pub(super) max_body_bytes: Option<usize>,
    pub(super) max_connections: Option<usize>,
    pub(super) max_requests_per_connection: Option<usize>,
    pub(super) read_timeout_ms: Option<u64>,
    pub(super) request_timeout_ms: Option<u64>,
    pub(super) write_timeout_ms: Option<u64>,
    pub(super) max_header_count: Option<usize>,
    pub(super) max_form_fields: Option<usize>,
    pub(super) max_form_field_bytes: Option<usize>,
    pub(super) max_json_depth: Option<usize>,
    pub(super) max_json_string_bytes: Option<usize>,
    pub(super) max_json_array_items: Option<usize>,
    pub(super) max_json_object_fields: Option<usize>,
    pub(super) max_instructions: Option<u64>,
    pub(super) max_runtime_alloc_bytes: Option<u64>,
    pub(super) session_ttl_secs: Option<u64>,
    pub(super) max_sessions: Option<usize>,
    pub(super) max_process_memory_bytes: Option<u64>,
    pub(super) resource_profiles_file: Option<String>,
}

#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct FileDomainLimits {
    pub(super) max_body_bytes: Option<usize>,
    pub(super) request_timeout_ms: Option<u64>,
    pub(super) max_form_fields: Option<usize>,
    pub(super) max_form_field_bytes: Option<usize>,
    pub(super) max_json_depth: Option<usize>,
    pub(super) max_json_string_bytes: Option<usize>,
    pub(super) max_json_array_items: Option<usize>,
    pub(super) max_json_object_fields: Option<usize>,
    pub(super) max_instructions: Option<u64>,
    pub(super) max_runtime_alloc_bytes: Option<u64>,
    pub(super) max_concurrent_requests: Option<usize>,
    pub(super) max_queued_requests: Option<usize>,
    pub(super) queue_timeout_ms: Option<u64>,
    pub(super) resource_profiles_file: Option<String>,
}

#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct FileDomainStorage {
    pub(super) data_root: Option<String>,
    pub(super) fs_mode: Option<String>,
    pub(super) max_upload_bytes: Option<u64>,
    pub(super) max_image_pixels: Option<u64>,
}

#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct FileDomainStatic {
    pub(super) root: Option<String>,
    pub(super) url_prefix: Option<String>,
    pub(super) max_asset_bytes: Option<u64>,
    pub(super) max_age_secs: Option<u64>,
    pub(super) immutable_max_age_secs: Option<u64>,
    pub(super) precompressed: Option<bool>,
}

#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct FileDomainTls {
    pub(super) cert_file: Option<String>,
    pub(super) key_file: Option<String>,
}

#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct FileDomain {
    pub(super) host: Option<String>,
    pub(super) aliases: Option<Vec<String>>,
    pub(super) config_file: Option<String>,
    pub(super) workdir: Option<String>,
    pub(super) app: Option<String>,
    pub(super) limits: FileDomainLimits,
    pub(super) storage: FileDomainStorage,
    pub(super) static_assets: FileDomainStatic,
    pub(super) tls: FileDomainTls,
    pub(super) reload: FileReload,
}

#[derive(Debug, Clone)]
pub(super) struct SourceReloadCliConfig {
    pub(super) enabled: bool,
    pub(super) poll_interval_ms: u64,
    pub(super) debounce_ms: u64,
    pub(super) debug_compile_errors: bool,
}

impl Default for SourceReloadCliConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_interval_ms: 1000,
            debounce_ms: 250,
            debug_compile_errors: false,
        }
    }
}

#[derive(Clone)]
pub(super) struct DomainTlsCliConfig {
    pub(super) cert_file: PathBuf,
    pub(super) key_file: PathBuf,
}

#[derive(Clone)]
pub(super) struct DomainCliConfig {
    pub(super) host: String,
    pub(super) aliases: Vec<String>,
    pub(super) workdir: PathBuf,
    pub(super) app: PathBuf,
    pub(super) config: ServerConfig,
    pub(super) storage: StorageCliConfig,
    pub(super) static_assets: StaticAssetsCliConfig,
    pub(super) resource_profiles_file: Option<PathBuf>,
    pub(super) max_concurrent_requests: usize,
    pub(super) max_queued_requests: usize,
    pub(super) queue_timeout_ms: u64,
    pub(super) tls: Option<DomainTlsCliConfig>,
    pub(super) reload: SourceReloadCliConfig,
}

pub(super) struct DomainRuntime {
    pub(super) host: Option<String>,
    pub(super) workdir: Option<PathBuf>,
    pub(super) app: PathBuf,
    pub(super) program: Arc<language_core::Program>,
    pub(super) native_runtime: Option<crate::native_runtime::SharedNativeRuntime>,
    pub(super) native_config: NativeCliConfig,
    pub(super) source_files: Arc<Vec<SourceFileState>>,
    pub(super) source_roots: Arc<Vec<PathBuf>>,
    pub(super) generation: u64,
    pub(super) config: ServerConfig,
    pub(super) appfs: Option<Arc<AppFs>>,
    pub(super) static_assets: Option<Arc<StaticAssets>>,
    pub(super) resource_profiles: Arc<ResourceProfiles>,
    pub(super) max_image_pixels: u64,
    pub(super) request_slots: Arc<Semaphore>,
    pub(super) queue_slots: Arc<Semaphore>,
    pub(super) queue_timeout_ms: u64,
    pub(super) max_concurrent_requests: usize,
    pub(super) max_queued_requests: usize,
    pub(super) storage_cli: StorageCliConfig,
    pub(super) static_cli: StaticAssetsCliConfig,
    pub(super) resource_profiles_file: Option<PathBuf>,
    pub(super) reload: SourceReloadCliConfig,
}

#[derive(Clone)]
pub(super) struct HostingRuntime {
    pub(super) default: Option<Arc<DomainRuntime>>,
    pub(super) domains: Arc<HashMap<String, Arc<DomainRuntime>>>,
}
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct FileCgroup {
    pub(super) dir: Option<String>,
    pub(super) memory_max_bytes: Option<u64>,
    pub(super) memory_swap_max_bytes: Option<u64>,
    pub(super) cpu_percent: Option<u32>,
    pub(super) pids_max: Option<u64>,
}

pub(super) fn validate_log_config(config: &LogConfig) -> Result<(), ServerConfigError> {
    for (name, path) in [
        ("logging.server_file", config.server_file.as_deref()),
        ("logging.access_file", config.access_file.as_deref()),
        ("logging.audit_file", config.audit_file.as_deref()),
    ] {
        if let Some(path) = path {
            if !path.is_absolute() {
                return Err(ServerConfigError::invalid(format!(
                    "config `{name}` must be an absolute path"
                )));
            }
            let parent = path
                .parent()
                .ok_or_else(|| ServerConfigError::invalid(format!("`{name}` has no parent")))?;
            if !parent.is_dir() {
                return Err(ServerConfigError::invalid(format!(
                    "log directory `{}` does not exist",
                    parent.display()
                )));
            }
            if let Ok(meta) = fs::symlink_metadata(path) {
                if meta.file_type().is_symlink() || !meta.is_file() {
                    return Err(ServerConfigError::invalid(format!(
                        "log target `{}` must be a regular non-symlink file",
                        path.display()
                    )));
                }
            }
        }
    }
    Ok(())
}

pub(super) fn config_abs_path(raw: &str, field: &str) -> Result<PathBuf, ServerConfigError> {
    let p = PathBuf::from(raw);
    if !p.is_absolute() {
        return Err(ServerConfigError::invalid(format!(
            "config `{field}` must be an absolute path"
        )));
    }
    Ok(p)
}

pub(super) fn read_server_config(path: &Path) -> Result<ServerFileConfig, ServerConfigError> {
    let meta = fs::symlink_metadata(path)
        .map_err(|e| ServerConfigError::io("inspect server config", path, e))?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(ServerConfigError::invalid(format!(
            "config file `{}` must be a regular non-symlink file",
            path.display()
        )));
    }
    if meta.len() > 1024 * 1024 {
        return Err(ServerConfigError::invalid(
            "server config file exceeds 1 MiB",
        ));
    }
    let text = fs::read_to_string(path)
        .map_err(|e| ServerConfigError::io("read server config", path, e))?;
    toml::from_str(&text).map_err(|source| ServerConfigError::Toml {
        kind: "server",
        path: path.to_path_buf(),
        source,
    })
}

fn read_domain_config(path: &Path) -> Result<FileDomain, ServerConfigError> {
    let meta = fs::symlink_metadata(path)
        .map_err(|e| ServerConfigError::io("inspect domain config", path, e))?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(ServerConfigError::invalid(format!(
            "domain config file `{}` must be a regular non-symlink file",
            path.display()
        )));
    }
    if meta.len() > 256 * 1024 {
        return Err(ServerConfigError::invalid(
            "domain config file exceeds 256 KiB",
        ));
    }
    let text = fs::read_to_string(path)
        .map_err(|e| ServerConfigError::io("read domain config", path, e))?;
    toml::from_str(&text).map_err(|source| ServerConfigError::Toml {
        kind: "domain",
        path: path.to_path_buf(),
        source,
    })
}

fn merge_domain_limits(base: FileDomainLimits, over: FileDomainLimits) -> FileDomainLimits {
    FileDomainLimits {
        max_body_bytes: over.max_body_bytes.or(base.max_body_bytes),
        request_timeout_ms: over.request_timeout_ms.or(base.request_timeout_ms),
        max_form_fields: over.max_form_fields.or(base.max_form_fields),
        max_form_field_bytes: over.max_form_field_bytes.or(base.max_form_field_bytes),
        max_json_depth: over.max_json_depth.or(base.max_json_depth),
        max_json_string_bytes: over.max_json_string_bytes.or(base.max_json_string_bytes),
        max_json_array_items: over.max_json_array_items.or(base.max_json_array_items),
        max_json_object_fields: over.max_json_object_fields.or(base.max_json_object_fields),
        max_instructions: over.max_instructions.or(base.max_instructions),
        max_runtime_alloc_bytes: over
            .max_runtime_alloc_bytes
            .or(base.max_runtime_alloc_bytes),
        max_concurrent_requests: over
            .max_concurrent_requests
            .or(base.max_concurrent_requests),
        max_queued_requests: over.max_queued_requests.or(base.max_queued_requests),
        queue_timeout_ms: over.queue_timeout_ms.or(base.queue_timeout_ms),
        resource_profiles_file: over.resource_profiles_file.or(base.resource_profiles_file),
    }
}

fn merge_domain_storage(base: FileDomainStorage, over: FileDomainStorage) -> FileDomainStorage {
    FileDomainStorage {
        data_root: over.data_root.or(base.data_root),
        fs_mode: over.fs_mode.or(base.fs_mode),
        max_upload_bytes: over.max_upload_bytes.or(base.max_upload_bytes),
        max_image_pixels: over.max_image_pixels.or(base.max_image_pixels),
    }
}

fn merge_domain_static(base: FileDomainStatic, over: FileDomainStatic) -> FileDomainStatic {
    FileDomainStatic {
        root: over.root.or(base.root),
        url_prefix: over.url_prefix.or(base.url_prefix),
        max_asset_bytes: over.max_asset_bytes.or(base.max_asset_bytes),
        max_age_secs: over.max_age_secs.or(base.max_age_secs),
        immutable_max_age_secs: over.immutable_max_age_secs.or(base.immutable_max_age_secs),
        precompressed: over.precompressed.or(base.precompressed),
    }
}

fn merge_domain(base: FileDomain, over: FileDomain) -> FileDomain {
    FileDomain {
        host: over.host.or(base.host),
        aliases: over.aliases.or(base.aliases),
        config_file: None,
        workdir: over.workdir.or(base.workdir),
        app: over.app.or(base.app),
        limits: merge_domain_limits(base.limits, over.limits),
        storage: merge_domain_storage(base.storage, over.storage),
        static_assets: merge_domain_static(base.static_assets, over.static_assets),
        tls: FileDomainTls {
            cert_file: over.tls.cert_file.or(base.tls.cert_file),
            key_file: over.tls.key_file.or(base.tls.key_file),
        },
        reload: FileReload {
            enabled: over.reload.enabled.or(base.reload.enabled),
            poll_interval_ms: over
                .reload
                .poll_interval_ms
                .or(base.reload.poll_interval_ms),
            debounce_ms: over.reload.debounce_ms.or(base.reload.debounce_ms),
            debug_compile_errors: over
                .reload
                .debug_compile_errors
                .or(base.reload.debug_compile_errors),
        },
    }
}

fn domain_path(workdir: &Path, raw: &str, field: &str) -> Result<PathBuf, ServerConfigError> {
    use std::path::Component;
    let rel = Path::new(raw);
    if rel.is_absolute()
        || rel.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(ServerConfigError::invalid(format!(
            "domain `{field}` must be a relative path inside workdir"
        )));
    }
    Ok(workdir.join(rel))
}

pub(super) fn build_domain_configs(
    entries: Vec<FileDomain>,
    global: &ServerConfig,
    global_storage: &StorageCliConfig,
    global_static: &StaticAssetsCliConfig,
    global_profiles: Option<&Path>,
    global_reload: &SourceReloadCliConfig,
) -> Result<Vec<DomainCliConfig>, ServerConfigError> {
    let mut out = Vec::with_capacity(entries.len());
    let mut seen = HashSet::new();
    let mut seen_workdirs = HashSet::new();
    for inline in entries {
        let included = match inline.config_file.as_deref() {
            Some(raw) => read_domain_config(&config_abs_path(raw, "domains.config_file")?)?,
            None => FileDomain::default(),
        };
        if included.config_file.is_some() {
            return Err(ServerConfigError::invalid(
                "nested domain config includes are not allowed",
            ));
        }
        if let (Some(a), Some(b)) = (inline.host.as_deref(), included.host.as_deref()) {
            if validate_public_host(a)? != validate_public_host(b)? {
                return Err(ServerConfigError::invalid(
                    "domain host in included config does not match server.toml",
                ));
            }
        }
        let merged = merge_domain(included, inline);
        let host =
            validate_public_host(merged.host.as_deref().ok_or_else(|| {
                ServerConfigError::invalid("each [[domains]] entry requires host")
            })?)?;
        let mut aliases = Vec::new();
        for raw in merged.aliases.as_deref().unwrap_or(&[]) {
            let alias = validate_public_host(raw)?;
            if alias == host || aliases.iter().any(|v| v == &alias) {
                return Err(ServerConfigError::invalid(format!(
                    "domain `{host}` contains duplicate alias `{alias}`"
                )));
            }
            aliases.push(alias);
        }
        for name in std::iter::once(&host).chain(aliases.iter()) {
            if !seen.insert(name.clone()) {
                return Err(ServerConfigError::invalid(format!(
                    "duplicate domain host/alias `{name}`"
                )));
            }
        }
        let workdir = config_abs_path(
            merged.workdir.as_deref().ok_or_else(|| {
                ServerConfigError::invalid(format!("domain `{host}` requires workdir"))
            })?,
            "domains.workdir",
        )?;
        if !workdir.is_dir() {
            return Err(ServerConfigError::invalid(format!(
                "domain `{host}` workdir `{}` is not a directory",
                workdir.display()
            )));
        }
        let canonical_workdir = fs::canonicalize(&workdir)
            .map_err(|e| ServerConfigError::io("canonicalize domain workdir", &workdir, e))?;
        if !seen_workdirs.insert(canonical_workdir) {
            return Err(ServerConfigError::invalid(format!(
                "domain `{host}` reuses another domain's workdir `{}`",
                workdir.display()
            )));
        }
        let app = domain_path(&workdir, merged.app.as_deref().unwrap_or("main.vrn"), "app")?;
        let mut config = global.clone();
        if let Some(v) = merged.limits.max_body_bytes {
            config.max_body_bytes = v;
        }
        if let Some(v) = merged.limits.request_timeout_ms {
            config.request_timeout_ms = v;
        }
        if let Some(v) = merged.limits.max_form_fields {
            config.max_form_fields = v;
        }
        if let Some(v) = merged.limits.max_form_field_bytes {
            config.max_form_field_bytes = v;
        }
        if let Some(v) = merged.limits.max_json_depth {
            config.max_json_depth = v;
        }
        if let Some(v) = merged.limits.max_json_string_bytes {
            config.max_json_string_bytes = v;
        }
        if let Some(v) = merged.limits.max_json_array_items {
            config.max_json_array_items = v;
        }
        if let Some(v) = merged.limits.max_json_object_fields {
            config.max_json_object_fields = v;
        }
        if let Some(v) = merged.limits.max_instructions {
            config.max_instructions = v;
        }
        if let Some(v) = merged.limits.max_runtime_alloc_bytes {
            config.max_runtime_alloc_bytes = v;
        }
        let max_concurrent_requests = merged
            .limits
            .max_concurrent_requests
            .unwrap_or(global.max_connections);
        let max_queued_requests = merged
            .limits
            .max_queued_requests
            .unwrap_or(max_concurrent_requests.saturating_mul(2));
        let queue_timeout_ms = merged
            .limits
            .queue_timeout_ms
            .unwrap_or(global.request_timeout_ms.min(5_000));
        if config.max_body_bytes == 0
            || config.request_timeout_ms == 0
            || config.max_form_fields == 0
            || config.max_form_field_bytes == 0
            || config.max_json_depth == 0
            || config.max_json_string_bytes == 0
            || config.max_json_array_items == 0
            || config.max_json_object_fields == 0
            || config.max_instructions == 0
            || config.max_runtime_alloc_bytes == 0
            || max_concurrent_requests == 0
            || queue_timeout_ms == 0
        {
            return Err(ServerConfigError::invalid(format!(
                "domain `{host}` limits must be greater than zero"
            )));
        }
        if max_concurrent_requests > global.max_connections {
            return Err(ServerConfigError::invalid(format!(
                "domain `{host}` max_concurrent_requests {} exceeds global max_connections {}",
                max_concurrent_requests, global.max_connections
            )));
        }
        let mut storage = global_storage.clone();
        storage.data_root = match merged.storage.data_root.as_deref() {
            Some(v) => Some(domain_path(&workdir, v, "storage.data_root")?),
            None => None,
        };
        if let Some(v) = merged.storage.fs_mode {
            storage.fs_mode = v;
        }
        if let Some(v) = merged.storage.max_upload_bytes {
            storage.max_upload_bytes = v;
        }
        if let Some(v) = merged.storage.max_image_pixels {
            storage.max_image_pixels = v;
        }
        let mut static_assets = global_static.clone();
        static_assets.root = match merged.static_assets.root.as_deref() {
            Some(v) => Some(domain_path(&workdir, v, "static_assets.root")?),
            None => None,
        };
        if let Some(v) = merged.static_assets.url_prefix {
            static_assets.url_prefix = v;
        }
        if let Some(v) = merged.static_assets.max_asset_bytes {
            static_assets.max_asset_bytes = v;
        }
        if let Some(v) = merged.static_assets.max_age_secs {
            static_assets.regular_max_age_secs = v;
        }
        if let Some(v) = merged.static_assets.immutable_max_age_secs {
            static_assets.immutable_max_age_secs = v;
        }
        if let Some(v) = merged.static_assets.precompressed {
            static_assets.precompressed = v;
        }
        static_assets.url_prefix = validate_static_prefix(&static_assets.url_prefix)?;
        let resource_profiles_file = match merged.limits.resource_profiles_file.as_deref() {
            Some(v) => Some(domain_path(&workdir, v, "limits.resource_profiles_file")?),
            None => global_profiles.map(Path::to_path_buf),
        };
        if merged.tls.cert_file.is_some() != merged.tls.key_file.is_some() {
            return Err(ServerConfigError::invalid(format!(
                "domain `{host}` tls.cert_file and tls.key_file must be supplied together"
            )));
        }
        let tls = match (
            merged.tls.cert_file.as_deref(),
            merged.tls.key_file.as_deref(),
        ) {
            (Some(cert), Some(key)) => Some(DomainTlsCliConfig {
                cert_file: config_abs_path(cert, "domains.tls.cert_file")?,
                key_file: config_abs_path(key, "domains.tls.key_file")?,
            }),
            _ => None,
        };
        let reload = SourceReloadCliConfig {
            enabled: merged.reload.enabled.unwrap_or(global_reload.enabled),
            poll_interval_ms: merged
                .reload
                .poll_interval_ms
                .unwrap_or(global_reload.poll_interval_ms),
            debounce_ms: merged
                .reload
                .debounce_ms
                .unwrap_or(global_reload.debounce_ms),
            debug_compile_errors: merged
                .reload
                .debug_compile_errors
                .unwrap_or(global_reload.debug_compile_errors),
        };
        if reload.poll_interval_ms == 0 || reload.debounce_ms == 0 {
            return Err(ServerConfigError::invalid(format!(
                "domain `{host}` reload poll_interval_ms and debounce_ms must be greater than zero"
            )));
        }
        out.push(DomainCliConfig {
            host,
            aliases,
            workdir,
            app,
            config,
            storage,
            static_assets,
            resource_profiles_file,
            max_concurrent_requests,
            max_queued_requests,
            queue_timeout_ms,
            tls,
            reload,
        });
    }
    Ok(out)
}
