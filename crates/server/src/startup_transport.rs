use crate::AuthRuntime;
use crate::backend_support::{bind_application_listener, try_reload_hosting};
use crate::bootstrap_config::{PublicPageCache, json_log_escape};
use crate::connection::{ConnectionServices, handle_connection};
use crate::operations::{run_http_redirect_listener, run_metrics_listener, shutdown_signal};
use crate::outbound_runtime::ServerOutbound;
use crate::rate_limit::RouteRateLimiter;
use crate::server_config_file::{DomainCliConfig, HostingRuntime};
use crate::server_errors::{ConnectionError, StartupError};
use crate::{
    CacheCliConfig, LifecycleCliConfig, ObservabilityCliConfig, TlsCliConfig, WebSecurityCliConfig,
};
use auth::SessionBackend;
use data::{Database, RedisStore};
use language_core::ServerConfig;
use observability::{Metrics, flush_logs, reopen_logs, server_event, server_log};
#[cfg(unix)]
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::{Semaphore, mpsc};
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::timeout;
use tokio_rustls::TlsAcceptor;

pub(super) struct TransportRuntime {
    pub(super) app: PathBuf,
    pub(super) config: ServerConfig,
    pub(super) tls: TlsCliConfig,
    pub(super) web: WebSecurityCliConfig,
    pub(super) lifecycle: LifecycleCliConfig,
    pub(super) observability: ObservabilityCliConfig,
    pub(super) cache: CacheCliConfig,
    pub(super) domains: Vec<DomainCliConfig>,
    pub(super) unix_socket: Option<PathBuf>,
    pub(super) behind_proxy: bool,
    pub(super) reverse_proxy_https: bool,
    pub(super) multi_domain: bool,
    pub(super) hosting: Arc<RwLock<HostingRuntime>>,
    pub(super) database: Option<Arc<Database>>,
    pub(super) sessions: SessionBackend,
    pub(super) auth_runtime: Arc<AuthRuntime>,
    pub(super) route_rate_limiter: Arc<RouteRateLimiter>,
    pub(super) public_cache: Arc<PublicPageCache>,
    pub(super) idempotency_redis: Option<RedisStore>,
    pub(super) outbound: Option<Arc<ServerOutbound>>,
    pub(super) metrics: Arc<Metrics>,
    pub(super) source_reload_task: Option<JoinHandle<()>>,
    pub(super) tls_acceptor: Option<TlsAcceptor>,
}

pub(super) async fn serve(runtime: TransportRuntime) -> Result<(), StartupError> {
    let metrics_task =
        spawn_metrics_listener(&runtime.observability, &runtime.metrics, &runtime.config);
    let redirect_task = spawn_redirect_listener(&runtime.tls, &runtime.config);
    let listener =
        bind_application_listener(runtime.unix_socket.as_deref(), runtime.config.listen).await?;
    log_transport_startup(&runtime);

    let TransportRuntime {
        app: _,
        config,
        tls,
        web,
        lifecycle,
        observability,
        cache,
        domains: _,
        unix_socket,
        behind_proxy,
        reverse_proxy_https: _,
        multi_domain: _,
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
    } = runtime;

    let slots = Arc::new(Semaphore::new(config.max_connections));
    let (hup_tx, mut hup_rx) = mpsc::unbounded_channel::<()>();
    crate::startup_lifecycle::install_hup_handler(hup_tx);
    let mut connections = JoinSet::new();

    loop {
        tokio::select! {
            signal = shutdown_signal() => {
                signal?;
                server_log("shutdown signal received; stopping accepts");
                break;
            }
            Some(()) = hup_rx.recv() => {
                match reopen_logs() {
                    Ok(()) => server_log("{\"event\":\"logs_reopened\"}"),
                    Err(err) => server_log(&format!("{{\"event\":\"log_reopen_failed\",\"error\":\"{}\"}}", json_log_escape(&err.to_string()))),
                }
                if behind_proxy {
                    try_reload_hosting(&hosting, &lifecycle, &route_rate_limiter, &cache, &auth_runtime);
                }
            }
            joined = connections.join_next(), if !connections.is_empty() => {
                if let Some(Err(err)) = joined {
                    server_event("error", "connection_join_failed", "http", &err.to_string());
                }
            }
            accepted = listener.accept() => {
                let (stream, peer_ip, peer_label) = match accepted {
                    Ok(value) => value,
                    Err(err) => {
                        server_event("error", "accept_failed", "http", &err.to_string());
                        continue;
                    }
                };
                let permit = match Arc::clone(&slots).acquire_owned().await {
                    Ok(value) => value,
                    Err(_) => break,
                };
                let hosting = Arc::clone(&hosting);
                let config = config.clone();
                let sessions = sessions.clone();
                let database = database.clone();
                let auth_runtime = Arc::clone(&auth_runtime);
                let tls_acceptor = tls_acceptor.clone();
                let handshake_timeout = Duration::from_millis(tls.handshake_timeout_ms);
                let expected_host = tls.public_host.clone();
                let web = web.clone();
                let lifecycle = lifecycle.clone();
                let route_rate_limiter = Arc::clone(&route_rate_limiter);
                let public_cache = Arc::clone(&public_cache);
                let idempotency_redis = idempotency_redis.clone();
                let outbound = outbound.clone();
                let metrics = Arc::clone(&metrics);
                let observability = observability.clone();
                connections.spawn(async move {
                    let _permit = permit;
                    let services = ConnectionServices {
                        hosting: &hosting,
                        config: &config,
                        sessions: &sessions,
                        database: database.as_deref(),
                        auth_runtime: &auth_runtime,
                        lifecycle: &lifecycle,
                        route_rate_limiter: &route_rate_limiter,
                        public_cache: &public_cache,
                        idempotency_redis: idempotency_redis.as_ref(),
                        outbound: outbound
                            .as_deref()
                            .map(|value| value as &dyn runtime::OutboundRuntime),
                        metrics: &metrics,
                        observability: &observability,
                        web: &web,
                    };
                    let result: Result<(), ConnectionError> = if let Some(acceptor) = tls_acceptor {
                        match timeout(handshake_timeout, acceptor.accept(stream)).await {
                            Err(_) => Err(ConnectionError::from(io::Error::new(io::ErrorKind::TimedOut, "TLS handshake timeout"))),
                            Ok(Err(err)) => Err(ConnectionError::from(err)),
                            Ok(Ok(tls_stream)) => {
                                let sni_host = tls_stream.get_ref().1.server_name().map(str::to_owned);
                                let multi_domain = hosting.read().map(|hosting| !hosting.domains.is_empty()).unwrap_or(true);
                                let pinned_host = if multi_domain { sni_host.as_deref() } else { expected_host.as_deref() };
                                handle_connection(tls_stream, &services, peer_ip, true, pinned_host).await
                            }
                        }
                    } else {
                        handle_connection(stream, &services, peer_ip, false, expected_host.as_deref()).await
                    };
                    if let Err(err) = result {
                        server_event("error", "connection_failed", "http", &format!("peer={peer_label} error={err}"));
                    }
                });
            }
        }
    }

    drop(listener);
    #[cfg(unix)]
    if let Some(path) = unix_socket.as_deref() {
        let _ = fs::remove_file(path);
    }
    crate::startup_lifecycle::abort_task(redirect_task);
    crate::startup_lifecycle::abort_task(metrics_task);
    crate::startup_lifecycle::abort_task(source_reload_task);
    crate::startup_lifecycle::drain_connections(&mut connections, lifecycle.shutdown_grace_ms)
        .await;
    server_log("shutdown complete");
    let _ = flush_logs();
    Ok(())
}

fn spawn_metrics_listener(
    observability: &ObservabilityCliConfig,
    metrics: &Arc<Metrics>,
    config: &ServerConfig,
) -> Option<JoinHandle<()>> {
    observability.metrics_listen.map(|addr| {
        let metrics = Arc::clone(metrics);
        let config = config.clone();
        tokio::spawn(async move {
            if let Err(err) = run_metrics_listener(addr, metrics, config).await {
                server_event(
                    "error",
                    "metrics_listener_stopped",
                    "metrics",
                    &err.to_string(),
                );
            }
        })
    })
}

fn spawn_redirect_listener(tls: &TlsCliConfig, config: &ServerConfig) -> Option<JoinHandle<()>> {
    let addr = tls.http_redirect_listen?;
    let public_host = tls.public_host.clone()?;
    let config = config.clone();
    Some(tokio::spawn(async move {
        if let Err(err) = run_http_redirect_listener(addr, public_host, config).await {
            server_event(
                "error",
                "redirect_listener_stopped",
                "http",
                &err.to_string(),
            );
        }
    }))
}

fn log_transport_startup(runtime: &TransportRuntime) {
    if let Some(path) = runtime.unix_socket.as_deref() {
        server_log(&format!(
            "listening on unix:{} (behind-proxy)",
            path.display()
        ));
    } else {
        server_log(&format!(
            "listening on {}://{}",
            if runtime.tls_acceptor.is_some() {
                "https"
            } else {
                "http"
            },
            runtime.config.listen
        ));
    }
    if runtime.reverse_proxy_https {
        server_log(&format!(
            "trusted HTTPS reverse proxy mode: transport={}, public host {}",
            if runtime.unix_socket.is_some() {
                "unix-socket"
            } else {
                "loopback-tcp"
            },
            runtime.tls.public_host.as_deref().unwrap_or("multi-domain")
        ));
    }
    if let Some(addr) = runtime.tls.http_redirect_listen {
        server_log(&format!(
            "HTTP redirect listener: http://{addr} -> https://{}",
            runtime.tls.public_host.as_deref().unwrap_or("<missing>")
        ));
    }
    if runtime.multi_domain {
        for domain in &runtime.domains {
            server_log(&format!(
                "domain: host={} aliases={:?} workdir={}",
                domain.host,
                domain.aliases,
                domain.workdir.display()
            ));
        }
    } else {
        server_log(&format!("app: {}", runtime.app.display()));
    }
    if let Some(addr) = runtime.observability.metrics_listen {
        server_log(&format!("metrics: http://{addr}/metrics"));
    }
    server_log(&format!(
        "structured access log: {}",
        if runtime.observability.access_log {
            "enabled"
        } else {
            "disabled"
        }
    ));
    server_log(&format!(
        "auth sessions: {}",
        if matches!(&runtime.sessions, SessionBackend::Redis(_)) {
            "redis"
        } else {
            "memory (development)"
        }
    ));
}
