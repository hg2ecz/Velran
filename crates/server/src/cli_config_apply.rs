use crate::bootstrap_config::{json_log_escape, read_secret_file};
use crate::resource_limits::ResourceLimitConfig;
use crate::server_config_file::{
    FileDomain, NativeCliConfig, SourceReloadCliConfig, config_abs_path, read_server_config,
};
use crate::server_errors::CliParseError;
use crate::tls_support::validate_public_host;
use crate::web_security::valid_cors_origin;
use crate::{
    AuthCliConfig, CacheCliConfig, LifecycleCliConfig, ObservabilityCliConfig,
    StaticAssetsCliConfig, StorageCliConfig, TlsCliConfig, WebSecurityCliConfig,
    validate_reserved_path,
};
use language_core::ServerConfig;
use observability::{LogConfig, server_log};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
pub(super) struct LoadedCliConfig {
    pub(super) app: Option<PathBuf>,
    pub(super) config: ServerConfig,
    pub(super) db_url: Option<String>,
    pub(super) resource_profiles_file: Option<PathBuf>,
    pub(super) domain_entries: Vec<FileDomain>,
    pub(super) web: WebSecurityCliConfig,
    pub(super) storage: StorageCliConfig,
    pub(super) static_assets: StaticAssetsCliConfig,
    pub(super) lifecycle: LifecycleCliConfig,
    pub(super) rate_limits_file: Option<PathBuf>,
    pub(super) allow_memory_rate_limit: bool,
    pub(super) observability: ObservabilityCliConfig,
    pub(super) log_config: LogConfig,
    pub(super) cache_cli: CacheCliConfig,
    pub(super) native: NativeCliConfig,
    pub(super) source_reload: SourceReloadCliConfig,
    pub(super) resource_limits: ResourceLimitConfig,
    pub(super) allow_insecure_db: bool,
    pub(super) auth: AuthCliConfig,
    pub(super) tls: TlsCliConfig,
    pub(super) unix_socket: Option<PathBuf>,
    pub(super) behind_proxy: bool,
}
pub(super) fn load(path: Option<&Path>) -> Result<LoadedCliConfig, CliParseError> {
    let mut app = None;
    let mut config = ServerConfig::default();
    let mut db_url = None;
    let mut resource_profiles_file = None;
    let mut domain_entries: Vec<FileDomain> = Vec::new();
    let mut web = WebSecurityCliConfig::default();
    let mut storage = StorageCliConfig::default();
    let mut static_assets = StaticAssetsCliConfig::default();
    let mut lifecycle = LifecycleCliConfig::default();
    let mut rate_limits_file = None;
    let mut allow_memory_rate_limit = false;
    let mut observability = ObservabilityCliConfig {
        metrics_listen: None,
        allow_public_metrics: false,
        access_log: true,
    };
    let mut log_config = LogConfig {
        server_file: None,
        access_file: None,
        audit_file: None,
        stderr: true,
    };
    let mut cache_cli = CacheCliConfig::default();
    let mut native = NativeCliConfig::default();
    let mut source_reload = SourceReloadCliConfig::default();
    let mut resource_limits = ResourceLimitConfig::default();
    let mut allow_insecure_db = false;
    let mut auth = AuthCliConfig {
        ldap_username_attribute: "uid".into(),
        login_max_attempts: 8,
        login_principal_max_attempts: 16,
        login_source_max_attempts: 64,
        mfa_max_attempts: 5,
        login_window_secs: 300,
        ..Default::default()
    };
    let mut tls = TlsCliConfig::default();
    let mut unix_socket: Option<PathBuf> = None;
    let mut behind_proxy = false;
    if let Some(path) = path {
        let file = read_server_config(path)?;
        if let Some(v) = file.server.app {
            app = Some(config_abs_path(&v, "server.app")?);
        }
        if let Some(v) = file.server.listen {
            config.listen = v.parse()?;
        }
        if let Some(v) = file.server.insecure_dev_cookies {
            config.insecure_dev_cookies = v;
        }
        if let Some(v) = file.server.unix_socket {
            unix_socket = Some(config_abs_path(&v, "server.unix_socket")?);
        }
        if let Some(v) = file.server.behind_proxy {
            behind_proxy = v;
        }
        if let Some(v) = file.server.expected_build_id {
            crate::build_identity::validate_expected(&v)?;
        }
        crate::native_config::apply_file(&mut native, file.native)?;
        if let Some(v) = file.tls.cert_file {
            tls.cert_file = Some(config_abs_path(&v, "tls.cert_file")?);
        }
        if let Some(v) = file.tls.key_file {
            tls.key_file = Some(config_abs_path(&v, "tls.key_file")?);
        }
        if let Some(v) = file.tls.handshake_timeout_ms {
            tls.handshake_timeout_ms = v;
        }
        if let Some(v) = file.tls.http_redirect_listen {
            tls.http_redirect_listen = Some(v.parse()?);
        }
        if let Some(v) = file.tls.public_host {
            tls.public_host = Some(validate_public_host(&v)?);
        }
        if let Some(v) = file.database.url_file {
            db_url = Some(read_secret_file(
                config_abs_path(&v, "database.url_file")?
                    .to_str()
                    .ok_or("non-UTF8 database.url_file path")?,
            )?);
        }
        if let Some(v) = file.database.allow_insecure {
            allow_insecure_db = v;
        }
        if let Some(v) = file.redis.url_file {
            auth.redis_url = Some(read_secret_file(
                config_abs_path(&v, "redis.url_file")?
                    .to_str()
                    .ok_or("non-UTF8 redis.url_file path")?,
            )?);
        }
        if let Some(v) = file.redis.allow_insecure {
            auth.allow_insecure_redis = v;
        }
        if let Some(v) = file.auth.ldap_url {
            auth.ldap_url = Some(v);
        }
        if let Some(v) = file.auth.ldap_search_base {
            auth.ldap_search_base = Some(v);
        }
        if let Some(v) = file.auth.ldap_username_attribute {
            auth.ldap_username_attribute = v;
        }
        if let Some(v) = file.auth.ldap_service_bind_dn_file {
            auth.ldap_bind_dn = Some(read_secret_file(
                config_abs_path(&v, "auth.ldap_service_bind_dn_file")?
                    .to_str()
                    .ok_or("non-UTF8 auth bind DN path")?,
            )?);
        }
        if let Some(v) = file.auth.ldap_service_bind_password_file {
            auth.ldap_bind_password = Some(read_secret_file(
                config_abs_path(&v, "auth.ldap_service_bind_password_file")?
                    .to_str()
                    .ok_or("non-UTF8 auth bind password path")?,
            )?);
        }
        if let Some(v) = file.auth.totp_secrets_file {
            auth.totp_secrets_file = Some(config_abs_path(&v, "auth.totp_secrets_file")?);
        }
        if let Some(v) = file.auth.roles_file {
            auth.roles_file = Some(config_abs_path(&v, "auth.roles_file")?);
        }
        if let Some(v) = file.auth.memberships_file {
            auth.memberships_file = Some(config_abs_path(&v, "auth.memberships_file")?);
        }
        if let Some(v) = file.auth.local_auth_db_url_file {
            auth.local_auth_db_url = Some(read_secret_file(
                config_abs_path(&v, "auth.local_auth_db_url_file")?
                    .to_str()
                    .ok_or("non-UTF8 local auth DB URL path")?,
            )?);
        }
        if let Some(v) = file.auth.require_totp {
            auth.require_totp = v;
        }
        if let Some(v) = file.auth.login_max_attempts {
            auth.login_max_attempts = v;
        }
        if let Some(v) = file.auth.login_principal_max_attempts {
            auth.login_principal_max_attempts = v;
        }
        if let Some(v) = file.auth.login_source_max_attempts {
            auth.login_source_max_attempts = v;
        }
        if let Some(v) = file.auth.mfa_max_attempts {
            auth.mfa_max_attempts = v;
        }
        if let Some(v) = file.auth.login_window_secs {
            auth.login_window_secs = v;
        }
        if let Some(v) = file.web.trusted_proxy_cidrs {
            web.trusted_proxy_cidrs = v
                .into_iter()
                .map(|x| {
                    x.parse().map_err(|err| {
                        CliParseError::invalid(format!("invalid trusted proxy CIDR `{x}`: {err}"))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
        }
        if let Some(v) = file.web.allow_missing_origin {
            web.allow_missing_origin = v;
        }
        if let Some(v) = file.web.cors_origins {
            for origin in &v {
                if !valid_cors_origin(origin) {
                    return Err(format!("invalid CORS origin `{origin}` in config").into());
                }
            }
            let mut seen = HashSet::new();
            if v.iter().any(|x| !seen.insert(x.clone())) {
                return Err("duplicate CORS origin in config".into());
            }
            web.cors_origins = v;
        }
        if let Some(v) = file.web.cors_allow_credentials {
            web.cors_allow_credentials = v;
        }
        if let Some(v) = file.web.webhook_secrets_dir {
            web.webhook_secrets_dir = Some(config_abs_path(&v, "web.webhook_secrets_dir")?);
        }
        if let Some(v) = file.web.egress_policy_file {
            web.egress_policy_file = Some(config_abs_path(&v, "web.egress_policy_file")?);
        }
        if let Some(v) = file.storage.data_root {
            storage.data_root = Some(config_abs_path(&v, "storage.data_root")?);
        }
        if let Some(v) = file.storage.fs_mode {
            storage.fs_mode = v;
        }
        if let Some(v) = file.storage.max_upload_bytes {
            storage.max_upload_bytes = v;
        }
        if let Some(v) = file.storage.max_image_pixels {
            storage.max_image_pixels = v;
        }
        if let Some(v) = file.static_assets.root {
            static_assets.root = Some(config_abs_path(&v, "static_assets.root")?);
        }
        if let Some(v) = file.static_assets.url_prefix {
            static_assets.url_prefix = v;
        }
        if let Some(v) = file.static_assets.max_asset_bytes {
            static_assets.max_asset_bytes = v;
        }
        if let Some(v) = file.static_assets.max_age_secs {
            static_assets.regular_max_age_secs = v;
        }
        if let Some(v) = file.static_assets.immutable_max_age_secs {
            static_assets.immutable_max_age_secs = v;
        }
        if let Some(v) = file.static_assets.precompressed {
            static_assets.precompressed = v;
        }
        if let Some(v) = file.lifecycle.health_live_path {
            lifecycle.live_path = validate_reserved_path(&v)?;
        }
        if let Some(v) = file.lifecycle.health_ready_path {
            lifecycle.ready_path = validate_reserved_path(&v)?;
        }
        if let Some(v) = file.lifecycle.health_dependency_timeout_ms {
            lifecycle.dependency_timeout_ms = v;
        }
        if let Some(v) = file.lifecycle.shutdown_grace_ms {
            lifecycle.shutdown_grace_ms = v;
        }
        if let Some(v) = file.observability.metrics_listen {
            observability.metrics_listen = Some(v.parse()?);
        }
        if let Some(v) = file.observability.allow_public_metrics {
            observability.allow_public_metrics = v;
        }
        if let Some(v) = file.observability.access_log {
            observability.access_log = v;
        }
        if let Some(v) = file.logging.server_file {
            log_config.server_file = Some(config_abs_path(&v, "logging.server_file")?);
        }
        if let Some(v) = file.logging.access_file {
            log_config.access_file = Some(config_abs_path(&v, "logging.access_file")?);
        }
        if let Some(v) = file.logging.audit_file {
            log_config.audit_file = Some(config_abs_path(&v, "logging.audit_file")?);
        }
        if let Some(v) = file.logging.stderr {
            log_config.stderr = v;
        }
        if let Some(v) = file.rate_limit.policies_file {
            rate_limits_file = Some(config_abs_path(&v, "rate_limit.policies_file")?);
        }
        if let Some(v) = file.rate_limit.allow_memory {
            allow_memory_rate_limit = v;
        }
        if let Some(v) = file.cache.max_ttl_secs {
            cache_cli.max_ttl_secs = v;
        }
        if let Some(v) = file.cache.max_entries {
            cache_cli.max_entries = v;
        }
        if let Some(v) = file.cache.max_bytes {
            cache_cli.max_bytes = v;
        }
        if let Some(v) = file.cache.allow_memory {
            cache_cli.allow_memory = v;
        }
        if let Some(v) = file.cache.singleflight_wait_timeout_ms {
            cache_cli.singleflight_wait_timeout_ms = v;
        }
        if let Some(v) = file.reload.enabled {
            source_reload.enabled = v;
        }
        if let Some(v) = file.reload.mode.as_deref() {
            source_reload.mode = crate::server_config_file::ReloadMode::parse(v)?;
        }
        if let Some(v) = file.reload.poll_interval_ms {
            source_reload.poll_interval_ms = v;
        }
        if let Some(v) = file.reload.debounce_ms {
            source_reload.debounce_ms = v;
        }
        if let Some(v) = file.reload.debug_compile_errors {
            source_reload.debug_compile_errors = v;
        }
        if let Some(v) = file.limits.max_header_bytes {
            config.max_header_bytes = v;
        }
        if let Some(v) = file.limits.max_body_bytes {
            config.max_body_bytes = v;
        }
        if let Some(v) = file.limits.max_connections {
            config.max_connections = v;
        }
        if let Some(v) = file.limits.max_requests_per_connection {
            config.max_requests_per_connection = v;
        }
        if let Some(v) = file.limits.read_timeout_ms {
            config.read_timeout_ms = v;
        }
        if let Some(v) = file.limits.request_timeout_ms {
            config.request_timeout_ms = v;
        }
        if let Some(v) = file.limits.write_timeout_ms {
            config.write_timeout_ms = v;
        }
        if let Some(v) = file.limits.max_header_count {
            config.max_header_count = v;
        }
        if let Some(v) = file.limits.max_form_fields {
            config.max_form_fields = v;
        }
        if let Some(v) = file.limits.max_form_field_bytes {
            config.max_form_field_bytes = v;
        }
        crate::cli_json_limits::apply(&file.limits, &mut config);
        if let Some(v) = file.limits.max_instructions {
            config.max_instructions = v;
        }
        if let Some(v) = file.limits.max_runtime_alloc_bytes {
            config.max_runtime_alloc_bytes = v;
        }
        if let Some(v) = file.limits.session_ttl_secs {
            config.session_ttl_secs = v;
        }
        if let Some(v) = file.limits.max_sessions {
            config.max_sessions = v;
        }
        if let Some(v) = file.limits.max_process_memory_bytes {
            resource_limits.max_address_space_bytes = Some(v);
        }
        if let Some(v) = file.limits.resource_profiles_file {
            resource_profiles_file = Some(config_abs_path(&v, "limits.resource_profiles_file")?);
        }
        if let Some(v) = file.cgroup.dir {
            resource_limits.cgroup_dir = Some(config_abs_path(&v, "cgroup.dir")?);
        }
        if let Some(v) = file.cgroup.memory_max_bytes {
            resource_limits.cgroup_memory_max_bytes = Some(v);
        }
        if let Some(v) = file.cgroup.memory_swap_max_bytes {
            resource_limits.cgroup_memory_swap_max_bytes = Some(v);
        }
        if let Some(v) = file.cgroup.cpu_percent {
            resource_limits.cgroup_cpu_percent = Some(v);
        }
        if let Some(v) = file.cgroup.pids_max {
            resource_limits.cgroup_pids_max = Some(v);
        }
        domain_entries = file.domains;
        server_log(&format!(
            "{{\"event\":\"server_config_loaded\",\"path\":\"{}\"}}",
            json_log_escape(&path.display().to_string())
        ));
    }
    Ok(LoadedCliConfig {
        app,
        config,
        db_url,
        resource_profiles_file,
        domain_entries,
        web,
        storage,
        static_assets,
        lifecycle,
        rate_limits_file,
        allow_memory_rate_limit,
        observability,
        log_config,
        cache_cli,
        native,
        source_reload,
        resource_limits,
        allow_insecure_db,
        auth,
        tls,
        unix_socket,
        behind_proxy,
    })
}
