use observability::{server_event, server_log};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::timeout;

pub(super) fn install_hup_handler(hup_tx: mpsc::UnboundedSender<()>) {
    #[cfg(unix)]
    let _hup_task = tokio::spawn(async move {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::hangup()) {
            Ok(mut hup) => {
                while hup.recv().await.is_some() {
                    if hup_tx.send(()).is_err() {
                        break;
                    }
                }
            }
            Err(err) => server_event(
                "error",
                "sighup_handler_failed",
                "logging",
                &err.to_string(),
            ),
        }
    });
    #[cfg(not(unix))]
    drop(hup_tx);
}

pub(super) fn abort_task(task: Option<JoinHandle<()>>) {
    if let Some(task) = task {
        task.abort();
    }
}

pub(super) async fn drain_connections(connections: &mut JoinSet<()>, shutdown_grace_ms: u64) {
    server_log(&format!(
        "draining {} active connection(s)",
        connections.len()
    ));
    let drain = async {
        while let Some(joined) = connections.join_next().await {
            if let Err(err) = joined {
                server_event("error", "connection_join_failed", "http", &err.to_string());
            }
        }
    };
    if timeout(Duration::from_millis(shutdown_grace_ms), drain)
        .await
        .is_err()
    {
        server_log("shutdown grace period expired; aborting remaining connections");
        connections.abort_all();
        while connections.join_next().await.is_some() {}
    }
}
