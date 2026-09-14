use crate::bootstrap_config::{
    audit_resource_profiles, load_resource_profiles, validate_route_rate_policies,
};
use crate::rate_limit::RouteRateLimiter;
use crate::server_config_file::{
    DomainCliConfig, DomainRuntime, HostingRuntime, NativeCliConfig, SourceReloadCliConfig,
};
use crate::server_errors::BackendSupportError;
use crate::static_delivery::{route_conflicts_static, validate_static_prefix};
use crate::{
    AuthRuntime, CacheCliConfig, LifecycleCliConfig, StaticAssets, StaticAssetsCliConfig,
    StorageCliConfig, cli, route_matches_exact_path, source_reload,
};
use compiler::compile_file_with_dependencies;
use language_core::{RouteAuth, ServerConfig};
use observability::{server_event, server_log};
use runtime::ExecutionLimits;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use storage::{AppFs, FsLimits, FsMode};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
#[cfg(unix)]
use tokio::net::UnixListener;
use tokio::sync::Semaphore;

pub(super) trait ServerIo: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> ServerIo for T {}
pub(super) type BoxedServerIo = Box<dyn ServerIo>;

pub(super) enum BoundListener {
    Tcp(TcpListener),
    #[cfg(unix)]
    Unix(UnixListener),
}

impl BoundListener {
    pub(super) async fn accept(&self) -> io::Result<(BoxedServerIo, IpAddr, String)> {
        match self {
            BoundListener::Tcp(listener) => {
                let (stream, peer) = listener.accept().await?;
                stream.set_nodelay(true)?;
                Ok((Box::new(stream), peer.ip(), peer.to_string()))
            }
            #[cfg(unix)]
            BoundListener::Unix(listener) => {
                let (stream, _) = listener.accept().await?;
                Ok((
                    Box::new(stream),
                    IpAddr::from([127, 0, 0, 1]),
                    "unix".into(),
                ))
            }
        }
    }
}

pub(super) async fn bind_application_listener(
    unix_socket: Option<&Path>,
    tcp_addr: std::net::SocketAddr,
) -> Result<BoundListener, BackendSupportError> {
    if let Some(path) = unix_socket {
        #[cfg(unix)]
        {
            use std::os::unix::fs::{FileTypeExt, PermissionsExt};
            if let Ok(meta) = fs::symlink_metadata(path) {
                if meta.file_type().is_symlink() || !meta.file_type().is_socket() {
                    return Err(BackendSupportError::UnsafeUnixSocketPath(
                        path.to_path_buf(),
                    ));
                }
                fs::remove_file(path).map_err(|source| {
                    BackendSupportError::io(
                        "remove Unix listener",
                        format!("`{}`", path.display()),
                        source,
                    )
                })?;
            }
            let listener = UnixListener::bind(path).map_err(|source| {
                BackendSupportError::io(
                    "bind Unix listener",
                    format!("`{}`", path.display()),
                    source,
                )
            })?;
            fs::set_permissions(path, fs::Permissions::from_mode(0o660)).map_err(|source| {
                BackendSupportError::io(
                    "set Unix listener permissions on",
                    format!("`{}`", path.display()),
                    source,
                )
            })?;
            return Ok(BoundListener::Unix(listener));
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            return Err(BackendSupportError::UnixSocketsUnsupported);
        }
    }
    Ok(BoundListener::Tcp(
        TcpListener::bind(tcp_addr).await.map_err(|source| {
            BackendSupportError::io("bind TCP listener", tcp_addr.to_string(), source)
        })?,
    ))
}

pub(super) fn prepare_domain_runtime(
    host: Option<String>,
    workdir: Option<PathBuf>,
    app: &Path,
    config: ServerConfig,
    storage_cli: StorageCliConfig,
    static_cli: StaticAssetsCliConfig,
    resource_profiles_file: Option<&Path>,
    max_concurrent_requests: usize,
    max_queued_requests: usize,
    queue_timeout_ms: u64,
    lifecycle: &LifecycleCliConfig,
    reload: SourceReloadCliConfig,
    native: &NativeCliConfig,
    generation: u64,
) -> Result<Arc<DomainRuntime>, BackendSupportError> {
    let compiled = compile_file_with_dependencies(app)?;
    // Observe the whole Velran source root rather than only the dependency files
    // known at the last successful compile. This makes create/delete/rename of a
    // dropped-in `.vrn` module part of the live lifecycle.
    let source_roots = Arc::new(compiled.source_roots.clone());
    let source_files = Arc::new(source_reload::snapshot_source_roots(source_roots.as_ref()));
    let program = Arc::new(compiled.program);
    let native_runtime = crate::native_runtime::initialize(app, native)?;
    let execution_limits = ExecutionLimits {
        max_instructions: config.max_instructions,
        max_allocated_bytes: config.max_runtime_alloc_bytes,
        max_external_io_bytes: config.max_runtime_alloc_bytes,
    };
    let resource_profiles = Arc::new(load_resource_profiles(
        resource_profiles_file,
        &execution_limits,
        max_concurrent_requests,
    )?);
    audit_resource_profiles(&program, &resource_profiles)?;
    let static_assets = if let Some(root) = static_cli.root.as_deref() {
        let fs = AppFs::open_root(
            root,
            FsMode {
                read: true,
                write: false,
                create: false,
            },
            FsLimits {
                max_file_bytes: static_cli.max_asset_bytes,
                ..FsLimits::default()
            },
        )?;
        Some(Arc::new(StaticAssets {
            fs,
            url_prefix: validate_static_prefix(&static_cli.url_prefix)?,
            regular_max_age_secs: static_cli.regular_max_age_secs,
            immutable_max_age_secs: static_cli.immutable_max_age_secs,
            precompressed: static_cli.precompressed,
        }))
    } else {
        None
    };
    if let Some(static_assets) = static_assets.as_deref() {
        if let Some(route) = program
            .routes
            .iter()
            .find(|r| route_conflicts_static(r, &static_assets.url_prefix))
        {
            return Err(BackendSupportError::StaticRouteConflict {
                route: route.path.clone(),
                prefix: static_assets.url_prefix.clone(),
            });
        }
    }
    if program
        .routes
        .iter()
        .any(|r| r.path.starts_with("/__velran/media/"))
    {
        return Err(BackendSupportError::ReservedMediaRoute);
    }
    for health_path in [&lifecycle.live_path, &lifecycle.ready_path] {
        if program
            .routes
            .iter()
            .any(|r| route_matches_exact_path(r, health_path))
        {
            return Err(BackendSupportError::ReservedHealthRoute {
                path: health_path.to_string(),
            });
        }
        if let Some(static_assets) = static_assets.as_deref() {
            if health_path.starts_with(&static_assets.url_prefix) {
                return Err(BackendSupportError::HealthStaticConflict {
                    path: health_path.to_string(),
                    prefix: static_assets.url_prefix.clone(),
                });
            }
        }
    }
    let appfs = if program.routes.iter().any(|r| r.upload.is_some()) {
        let root = storage_cli
            .data_root
            .as_deref()
            .ok_or(BackendSupportError::MissingUploadDataRoot)?;
        let mode = FsMode::parse(&storage_cli.fs_mode)?;
        if !mode.create || !mode.write {
            return Err(BackendSupportError::UploadPermissions);
        }
        if program
            .routes
            .iter()
            .any(|r| r.upload.as_ref().map(|u| u.image).unwrap_or(false))
            && !mode.read
        {
            return Err(BackendSupportError::ImageUploadPermissions);
        }
        Some(Arc::new(AppFs::open_root(
            root,
            mode,
            FsLimits {
                max_file_bytes: storage_cli.max_upload_bytes,
                ..FsLimits::default()
            },
        )?))
    } else {
        None
    };
    Ok(Arc::new(DomainRuntime {
        host,
        workdir,
        app: app.to_path_buf(),
        program,
        native_runtime,
        native_config: native.clone(),
        source_files,
        source_roots,
        generation,
        config,
        appfs,
        static_assets,
        resource_profiles,
        max_image_pixels: storage_cli.max_image_pixels,
        request_slots: Arc::new(Semaphore::new(max_concurrent_requests)),
        queue_slots: Arc::new(Semaphore::new(max_queued_requests)),
        queue_timeout_ms,
        max_concurrent_requests,
        max_queued_requests,
        storage_cli,
        static_cli,
        resource_profiles_file: resource_profiles_file.map(Path::to_path_buf),
        reload,
    }))
}

pub(super) fn build_hosting_runtime(
    app: &Path,
    config: &ServerConfig,
    storage_cli: &StorageCliConfig,
    static_cli: &StaticAssetsCliConfig,
    resource_profiles_file: Option<&Path>,
    lifecycle: &LifecycleCliConfig,
    domain_cli: &[DomainCliConfig],
    global_reload: &SourceReloadCliConfig,
    native: &NativeCliConfig,
) -> Result<HostingRuntime, BackendSupportError> {
    if domain_cli.is_empty() {
        return Ok(HostingRuntime {
            default: Some(prepare_domain_runtime(
                None,
                None,
                app,
                config.clone(),
                storage_cli.clone(),
                static_cli.clone(),
                resource_profiles_file,
                config.max_connections,
                config.max_connections.saturating_mul(2),
                config.request_timeout_ms.min(5_000),
                lifecycle,
                global_reload.clone(),
                native,
                1,
            )?),
            domains: Arc::new(HashMap::new()),
        });
    }
    let mut domains = HashMap::new();
    for d in domain_cli {
        let runtime = prepare_domain_runtime(
            Some(d.host.clone()),
            Some(d.workdir.clone()),
            &d.app,
            d.config.clone(),
            d.storage.clone(),
            d.static_assets.clone(),
            d.resource_profiles_file.as_deref(),
            d.max_concurrent_requests,
            d.max_queued_requests,
            d.queue_timeout_ms,
            lifecycle,
            d.reload.clone(),
            native,
            1,
        )?;
        domains.insert(d.host.clone(), Arc::clone(&runtime));
        for alias in &d.aliases {
            domains.insert(alias.clone(), Arc::clone(&runtime));
        }
    }
    Ok(HostingRuntime {
        default: None,
        domains: Arc::new(domains),
    })
}

pub(super) fn try_reload_hosting(
    hosting: &Arc<RwLock<HostingRuntime>>,
    lifecycle: &LifecycleCliConfig,
    route_rate_limiter: &RouteRateLimiter,
    cache_cli: &CacheCliConfig,
    auth_runtime: &AuthRuntime,
) {
    let reloaded = match cli::parse_args() {
        Ok(v) => v,
        Err(err) => {
            server_event(
                "error",
                "domain_reload_rejected",
                "reload",
                &err.to_string(),
            );
            return;
        }
    };
    let candidate = match build_hosting_runtime(
        &reloaded.app,
        &reloaded.config,
        &reloaded.storage,
        &reloaded.static_assets,
        reloaded.resource_profiles_file.as_deref(),
        lifecycle,
        &reloaded.domains,
        &reloaded.source_reload,
        &reloaded.native,
    ) {
        Ok(v) => v,
        Err(err) => {
            server_event(
                "error",
                "domain_reload_rejected",
                "reload",
                &err.to_string(),
            );
            return;
        }
    };

    let mut unique = HashSet::new();
    let candidate_domains: Vec<_> = candidate
        .default
        .iter()
        .cloned()
        .chain(candidate.domains.values().cloned())
        .filter(|d| {
            d.host
                .as_ref()
                .map(|h| unique.insert(h.clone()))
                .unwrap_or(true)
        })
        .collect();

    let mut validation_error = None;
    for domain in &candidate_domains {
        if let Err(err) =
            validate_route_rate_policies(&domain.program, route_rate_limiter.policies.as_ref())
        {
            validation_error = Some(err.to_string());
            break;
        }
        for route in &domain.program.routes {
            if route
                .public_cache
                .as_ref()
                .is_some_and(|public_cache| public_cache.ttl_secs > cache_cli.max_ttl_secs)
            {
                validation_error = Some(format!(
                    "domain {:?} route `{}` cache ttl exceeds configured operator maximum",
                    domain.host, route.name
                ));
                break;
            }
        }
        if validation_error.is_some() {
            break;
        }
    }
    if candidate_domains.iter().any(|d| {
        d.program
            .routes
            .iter()
            .any(|r| !matches!(r.auth, RouteAuth::Public | RouteAuth::Webhook(_)))
    }) && auth_runtime.ldap.is_none()
        && auth_runtime.local.is_none()
    {
        validation_error = Some(
            "reloaded applications require authentication but no authentication backend is active"
                .into(),
        );
    }

    if let Some(err) = validation_error {
        server_event("error", "domain_reload_rejected", "reload", &err);
        return;
    }
    match hosting.write() {
        Ok(mut current) => {
            *current = candidate;
            server_log("{\"event\":\"domain_reload_committed\"}");
        }
        Err(_) => server_event(
            "error",
            "domain_reload_failed",
            "reload",
            "hosting runtime lock poisoned",
        ),
    }
}
