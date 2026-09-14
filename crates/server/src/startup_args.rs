use crate::resource_limits::ResourceLimitConfig;
use crate::server_config_file::{DomainCliConfig, NativeCliConfig, SourceReloadCliConfig};
use crate::{
    AuthCliConfig, CacheCliConfig, LifecycleCliConfig, ObservabilityCliConfig,
    StaticAssetsCliConfig, StorageCliConfig, TlsCliConfig, WebSecurityCliConfig,
};
use data::DbConfig;
use language_core::ServerConfig;
use observability::LogConfig;
use std::path::PathBuf;

pub(super) struct StartupArgs {
    pub(super) app: PathBuf,
    pub(super) config: ServerConfig,
    pub(super) db_config: Option<DbConfig>,
    pub(super) auth: AuthCliConfig,
    pub(super) tls: TlsCliConfig,
    pub(super) web: WebSecurityCliConfig,
    pub(super) storage: StorageCliConfig,
    pub(super) resource_limits: ResourceLimitConfig,
    pub(super) resource_profiles_file: Option<PathBuf>,
    pub(super) static_assets: StaticAssetsCliConfig,
    pub(super) lifecycle: LifecycleCliConfig,
    pub(super) rate_limits_file: Option<PathBuf>,
    pub(super) allow_memory_rate_limit: bool,
    pub(super) observability: ObservabilityCliConfig,
    pub(super) cache: CacheCliConfig,
    pub(super) native: NativeCliConfig,
    pub(super) log_config: LogConfig,
    pub(super) domains: Vec<DomainCliConfig>,
    pub(super) unix_socket: Option<PathBuf>,
    pub(super) behind_proxy: bool,
    pub(super) source_reload: SourceReloadCliConfig,
}
