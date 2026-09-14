use super::http_io::{read_request, read_request_head, write_response_with_timeout};
use super::{LifecycleCliConfig, Response};
use crate::response_headers::HeaderName;
use data::{Database, RedisStore};
use language_core::ServerConfig;
use observability::{Metrics, server_event};
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::timeout;

pub(super) fn install_panic_logging_hook() {
    std::panic::set_hook(Box::new(|info| {
        let thread = std::thread::current();
        let thread_name = thread.name().unwrap_or("unnamed");
        let payload = if let Some(v) = info.payload().downcast_ref::<&str>() {
            *v
        } else if let Some(v) = info.payload().downcast_ref::<String>() {
            v.as_str()
        } else {
            "non-string panic payload"
        };
        let location = info
            .location()
            .map(|v| format!("{}:{}:{}", v.file(), v.line(), v.column()))
            .unwrap_or_else(|| "unknown".to_string());
        server_event(
            "error",
            "thread_panic",
            "runtime",
            &format!("thread={thread_name} location={location} panic={payload}"),
        );
    }));
}

pub(super) async fn shutdown_signal() -> Result<(), io::Error> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut term = signal(SignalKind::terminate())?;
        tokio::select! {r=tokio::signal::ctrl_c()=>{r?;},_=term.recv()=>{}}
        Ok(())
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await?;
        Ok(())
    }
}

pub(super) async fn serve_health_endpoint(
    path: &str,
    method: &str,
    lifecycle: &LifecycleCliConfig,
    database: Option<&Database>,
    redis: Option<&RedisStore>,
) -> Response {
    if !matches!(method, "GET" | "HEAD") {
        return Response::text(405, "Method Not Allowed", b"method not allowed\n");
    }
    let mut response = if path == lifecycle.live_path.as_str() {
        Response::new(
            200,
            "OK",
            "application/json; charset=utf-8",
            br#"{"status":"live"}"#,
        )
    } else {
        let check = async {
            if let Some(db) = database {
                db.ping().await.map_err(|_| ())?;
            }
            if let Some(redis) = redis {
                redis.ping().await.map_err(|_| ())?;
            }
            Ok::<(), ()>(())
        };
        match timeout(
            Duration::from_millis(lifecycle.dependency_timeout_ms),
            check,
        )
        .await
        {
            Ok(Ok(())) => Response::new(
                200,
                "OK",
                "application/json; charset=utf-8",
                br#"{"status":"ready"}"#,
            ),
            _ => Response::new(
                503,
                "Service Unavailable",
                "application/json; charset=utf-8",
                br#"{"status":"not_ready"}"#,
            ),
        }
    };
    response.push_header(HeaderName::CacheControl, "no-store");
    if method == "HEAD" {
        response.content_length_override = Some(response.body.len());
        response.body.clear();
        response.suppress_body = true;
    }
    response
}

pub(super) async fn run_http_redirect_listener(
    addr: std::net::SocketAddr,
    public_host: String,
    config: ServerConfig,
) -> Result<(), io::Error> {
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (mut stream, _peer) = listener.accept().await?;
        let host = public_host.clone();
        let cfg = config.clone();
        tokio::spawn(async move {
            let _ = stream.set_nodelay(true);
            let mut buffer = Vec::with_capacity(2048);
            let request = match timeout(
                Duration::from_millis(cfg.read_timeout_ms),
                read_request(&mut stream, &mut buffer, &cfg),
            )
            .await
            {
                Ok(Ok(Some(v))) => v,
                _ => {
                    let _ = write_response_with_timeout(
                        &mut stream,
                        &cfg,
                        Response::text(400, "Bad Request", b"bad request\\n"),
                        false,
                        false,
                    )
                    .await;
                    return;
                }
            };
            if request.method != "GET" && request.method != "HEAD" {
                let _ = write_response_with_timeout(
                    &mut stream,
                    &cfg,
                    Response::text(405, "Method Not Allowed", b"use HTTPS\\n"),
                    false,
                    false,
                )
                .await;
                return;
            }
            let location = format!("https://{}{}", host, request.target);
            if !safe_redirect_location(&location) {
                let _ = write_response_with_timeout(
                    &mut stream,
                    &cfg,
                    Response::text(400, "Bad Request", b"bad request\\n"),
                    false,
                    false,
                )
                .await;
                return;
            }
            let mut response = Response::redirect(308, "Permanent Redirect", &location);
            if request.method == "HEAD" {
                response.body.clear();
            }
            let _ = write_response_with_timeout(&mut stream, &cfg, response, false, false).await;
        });
    }
}

pub(super) fn safe_redirect_location(v: &str) -> bool {
    !v.contains('\r') && !v.contains('\n') && v.starts_with("https://")
}

pub(super) async fn run_metrics_listener(
    addr: std::net::SocketAddr,
    metrics: Arc<Metrics>,
    config: ServerConfig,
) -> Result<(), io::Error> {
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (mut stream, _peer) = listener.accept().await?;
        let metrics = Arc::clone(&metrics);
        let cfg = config.clone();
        tokio::spawn(async move {
            let mut buffer = Vec::with_capacity(2048);
            let head = match timeout(
                Duration::from_millis(cfg.read_timeout_ms),
                read_request_head(&mut stream, &mut buffer, &cfg),
            )
            .await
            {
                Ok(Ok(Some(h))) => h,
                _ => return,
            };
            let path = head
                .target
                .split_once('?')
                .map(|v| v.0)
                .unwrap_or(head.target.as_str());
            let mut response = if path != "/metrics" {
                Response::text(404, "Not Found", b"not found\n")
            } else if !matches!(head.method.as_str(), "GET" | "HEAD") {
                Response::text(405, "Method Not Allowed", b"method not allowed\n")
            } else {
                let body = metrics.render_prometheus();
                Response::new(
                    200,
                    "OK",
                    "text/plain; version=0.0.4; charset=utf-8",
                    body.as_bytes(),
                )
            };
            if head.method == "HEAD" {
                response.content_length_override = Some(response.body.len());
                response.body.clear();
                response.suppress_body = true;
            }
            let _ = write_response_with_timeout(&mut stream, &cfg, response, false, false).await;
        });
    }
}
