use crate::LifecycleCliConfig;
use crate::bootstrap_config::{PublicPageCache, json_log_escape};
use crate::outbound_runtime::ServerOutbound;
use crate::rate_limit::RouteRateLimiter;
use crate::server_config_file::{DomainRuntime, HostingRuntime};
use crate::server_errors::SourceReloadError;
use crate::source_reload_candidate::build_candidate;
use observability::{server_event, server_log};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SourceFileState {
    pub(super) path: PathBuf,
    pub(super) modified: Option<SystemTime>,
    pub(super) len: Option<u64>,
    pub(super) sha256: Option<[u8; 32]>,
}

pub(super) fn snapshot_source_files(paths: &[PathBuf]) -> Vec<SourceFileState> {
    let mut states: Vec<_> = paths.iter().map(|path| source_state(path)).collect();
    states.sort_by(|a, b| a.path.cmp(&b.path));
    states.dedup_by(|a, b| a.path == b.path);
    states
}

pub(super) fn snapshot_source_roots(roots: &[PathBuf]) -> Vec<SourceFileState> {
    let mut paths = Vec::new();
    for root in roots {
        let mut pending = vec![root.clone()];
        while let Some(dir) = pending.pop() {
            let Ok(rd) = fs::read_dir(&dir) else { continue };
            let mut entries: Vec<_> = rd.filter_map(Result::ok).collect();
            entries.sort_by_key(|e| e.file_name());
            for item in entries {
                let path = item.path();
                let Ok(meta) = fs::symlink_metadata(&path) else {
                    continue;
                };
                if meta.file_type().is_symlink() {
                    if path.extension().and_then(|v| v.to_str()) == Some("vrn")
                        || path.file_name().and_then(|v| v.to_str()) == Some("velran.toml")
                    {
                        paths.push(path)
                    }
                    continue;
                }
                if meta.is_dir() {
                    pending.push(path)
                } else if meta.is_file()
                    && (path.extension().and_then(|v| v.to_str()) == Some("vrn")
                        || path.file_name().and_then(|v| v.to_str()) == Some("velran.toml"))
                {
                    paths.push(path)
                }
            }
        }
    }
    snapshot_source_files(&paths)
}

fn source_state(path: &PathBuf) -> SourceFileState {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => {
            let sha256 = fs::read(path).ok().map(|bytes| {
                let digest = Sha256::digest(bytes);
                let mut out = [0u8; 32];
                out.copy_from_slice(&digest);
                out
            });
            SourceFileState {
                path: path.clone(),
                modified: meta.modified().ok(),
                len: Some(meta.len()),
                sha256,
            }
        }
        _ => SourceFileState {
            path: path.clone(),
            modified: None,
            len: None,
            sha256: None,
        },
    }
}

fn resnapshot(roots: &[PathBuf]) -> Vec<SourceFileState> {
    snapshot_source_roots(roots)
}

struct WatchState {
    generation: u64,
    last_check: Instant,
    observed: Vec<SourceFileState>,
    pending_since: Option<Instant>,
    failed_observed: Option<Vec<SourceFileState>>,
    retry_after: Option<Instant>,
    retry_delay_ms: u64,
}

fn domain_key(domain: &DomainRuntime) -> String {
    domain.host.clone().unwrap_or_else(|| "<default>".into())
}

fn unique_domains(hosting: &HostingRuntime) -> Vec<Arc<DomainRuntime>> {
    let mut seen = HashSet::new();
    hosting
        .default
        .iter()
        .cloned()
        .chain(hosting.domains.values().cloned())
        .filter(|domain| {
            domain
                .host
                .as_ref()
                .map(|host| seen.insert(host.clone()))
                .unwrap_or(true)
        })
        .collect()
}

fn commit_candidate(
    hosting: &Arc<RwLock<HostingRuntime>>,
    old: &Arc<DomainRuntime>,
    candidate: Arc<DomainRuntime>,
) -> Result<bool, SourceReloadError> {
    let mut guard = hosting
        .write()
        .map_err(|_| SourceReloadError::HostingLockPoisoned)?;
    let mut replaced = false;

    if guard
        .default
        .as_ref()
        .map(|current| Arc::ptr_eq(current, old))
        .unwrap_or(false)
    {
        guard.default = Some(Arc::clone(&candidate));
        replaced = true;
    }

    if guard
        .domains
        .values()
        .any(|current| Arc::ptr_eq(current, old))
    {
        let mut domains = (*guard.domains).clone();
        for current in domains.values_mut() {
            if Arc::ptr_eq(current, old) {
                *current = Arc::clone(&candidate);
                replaced = true;
            }
        }
        guard.domains = Arc::new(domains);
    }
    Ok(replaced)
}

async fn invalidate_domain_cache(
    public_cache: &PublicPageCache,
    old: &DomainRuntime,
    candidate: &DomainRuntime,
) -> Result<(), SourceReloadError> {
    let namespace = candidate.host.as_deref().unwrap_or("__default__");
    let mut routes = HashSet::new();
    for route in old
        .program
        .routes
        .iter()
        .chain(candidate.program.routes.iter())
    {
        if route.public_cache.is_some() {
            routes.insert(route.name.clone());
        }
    }
    for route in routes {
        public_cache
            .invalidate_route(&format!("{namespace}:{route}"))
            .await?;
    }
    Ok(())
}

pub(super) fn spawn_source_reload_supervisor(
    hosting: Arc<RwLock<HostingRuntime>>,
    lifecycle: LifecycleCliConfig,
    route_rate_limiter: Arc<RouteRateLimiter>,
    public_cache: Arc<PublicPageCache>,
    cache_max_ttl_secs: u64,
    cache_available: bool,
    auth_enabled: bool,
    database_available: bool,
    idempotency_available: bool,
    webhook_secrets_available: bool,
    outbound: Option<Arc<ServerOutbound>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut states: HashMap<String, WatchState> = HashMap::new();
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let snapshot = match hosting.read() {
                Ok(value) => value.clone(),
                Err(_) => {
                    server_event(
                        "error",
                        "source_reload_supervisor_failed",
                        "reload",
                        "hosting runtime lock poisoned",
                    );
                    return;
                }
            };
            let domains = unique_domains(&snapshot);
            let live_keys: HashSet<String> =
                domains.iter().map(|domain| domain_key(domain)).collect();
            states.retain(|key, _| live_keys.contains(key));

            for domain in domains {
                if !domain.reload.enabled {
                    states.remove(&domain_key(&domain));
                    continue;
                }
                let key = domain_key(&domain);
                let now = Instant::now();
                let state = states.entry(key.clone()).or_insert_with(|| WatchState {
                    generation: domain.generation,
                    last_check: now,
                    observed: (*domain.source_files).clone(),
                    pending_since: None,
                    failed_observed: None,
                    retry_after: None,
                    retry_delay_ms: 2000,
                });
                if state.generation != domain.generation {
                    *state = WatchState {
                        generation: domain.generation,
                        last_check: now,
                        observed: (*domain.source_files).clone(),
                        pending_since: None,
                        failed_observed: None,
                        retry_after: None,
                        retry_delay_ms: 2000,
                    };
                    continue;
                }
                if now.duration_since(state.last_check)
                    < Duration::from_millis(domain.reload.poll_interval_ms)
                {
                    continue;
                }
                state.last_check = now;
                let current = resnapshot(domain.source_roots.as_ref());
                if current != state.observed {
                    state.observed = current;
                    state.pending_since = Some(now);
                    state.failed_observed = None;
                    state.retry_after = None;
                    state.retry_delay_ms = 2000;
                    server_log(&format!(
                        "{{\"event\":\"source_change_detected\",\"domain\":\"{}\",\"generation\":{},\"debounce_ms\":{}}}",
                        json_log_escape(&key),
                        domain.generation,
                        domain.reload.debounce_ms
                    ));
                    continue;
                }
                if state.observed == *domain.source_files {
                    state.pending_since = None;
                    continue;
                }
                let failed_same = state.failed_observed.as_ref() == Some(&state.observed);
                let retry_due = failed_same
                    && state
                        .retry_after
                        .map(|retry_at| now >= retry_at)
                        .unwrap_or(false);
                if failed_same && !retry_due {
                    continue;
                }
                if !retry_due {
                    let Some(pending_since) = state.pending_since else {
                        state.pending_since = Some(now);
                        continue;
                    };
                    if now.duration_since(pending_since)
                        < Duration::from_millis(domain.reload.debounce_ms)
                    {
                        continue;
                    }
                }

                let failed_snapshot = state.observed.clone();
                state.pending_since = None;
                let _ = state;

                let candidate_generation = domain.generation.saturating_add(1);
                server_log(&format!(
                    "{{\"event\":\"reload_candidate_started\",\"domain\":\"{}\",\"current_generation\":{},\"candidate_generation\":{},\"mode\":\"{}\"}}",
                    json_log_escape(&key),
                    domain.generation,
                    candidate_generation,
                    domain.reload.mode.as_str()
                ));

                let old: Arc<DomainRuntime> = Arc::clone(&domain);
                let lifecycle_for_build = lifecycle.clone();
                let limiter_for_build = Arc::clone(&route_rate_limiter);
                let outbound_for_build = outbound.clone();
                let build = tokio::task::spawn_blocking(move || {
                    build_candidate(
                        &old,
                        &lifecycle_for_build,
                        &limiter_for_build,
                        cache_max_ttl_secs,
                        cache_available,
                        auth_enabled,
                        database_available,
                        idempotency_available,
                        webhook_secrets_available,
                        outbound_for_build.as_deref(),
                    )
                    .map(|candidate: Arc<DomainRuntime>| -> (Arc<DomainRuntime>, Arc<DomainRuntime>) { (old, candidate) })
                })
                .await;

                match build {
                    Ok(Ok((old, candidate))) => {
                        let after_build = resnapshot(old.source_roots.as_ref());
                        if after_build != failed_snapshot {
                            server_log(&format!(
                                "{{\"event\":\"reload_candidate_source_changed\",\"domain\":\"{}\",\"candidate_generation\":{},\"action\":\"discard_and_retry\"}}",
                                json_log_escape(&key),
                                candidate.generation
                            ));
                            if let Some(state) = states.get_mut(&key) {
                                state.observed = after_build;
                                state.pending_since = Some(Instant::now());
                                state.failed_observed = None;
                                state.retry_after = None;
                                state.retry_delay_ms = 2000;
                            }
                            continue;
                        }
                        server_log(&format!(
                            "{{\"event\":\"reload_candidate_ready\",\"domain\":\"{}\",\"candidate_generation\":{},\"source_files\":{}}}",
                            json_log_escape(&key),
                            candidate.generation,
                            candidate.source_files.len()
                        ));
                        if let Err(err) =
                            invalidate_domain_cache(&public_cache, &old, &candidate).await
                        {
                            server_event(
                                "error",
                                "source_reload_cache_invalidation_failed",
                                "reload",
                                &format!("domain={key} error={err}"),
                            );
                            if let Some(state) = states.get_mut(&key) {
                                state.failed_observed = Some(failed_snapshot.clone());
                                state.pending_since = None;
                                state.retry_after = Some(
                                    Instant::now() + Duration::from_millis(state.retry_delay_ms),
                                );
                                state.retry_delay_ms =
                                    state.retry_delay_ms.saturating_mul(2).min(60_000);
                            }
                            continue;
                        }
                        match commit_candidate(&hosting, &old, Arc::clone(&candidate)) {
                            Ok(true) => {
                                if domain.reload.debug_compile_errors {
                                    crate::dev_compile_error::clear(&key);
                                }
                                server_log(&format!(
                                    "{{\"event\":\"reload_activated\",\"domain\":\"{}\",\"old_generation\":{},\"new_generation\":{},\"source_files\":{}}}",
                                    json_log_escape(&key),
                                    old.generation,
                                    candidate.generation,
                                    candidate.source_files.len()
                                ));
                                states.remove(&key);
                            }
                            Ok(false) => {
                                server_log(&format!(
                                    "{{\"event\":\"source_reload_stale\",\"domain\":\"{}\",\"generation\":{}}}",
                                    json_log_escape(&key),
                                    old.generation
                                ));
                                states.remove(&key);
                            }
                            Err(err) => {
                                server_event(
                                    "error",
                                    "source_reload_commit_failed",
                                    "reload",
                                    &format!("domain={key} error={err}"),
                                );
                                if let Some(state) = states.get_mut(&key) {
                                    state.failed_observed = Some(failed_snapshot.clone());
                                    state.pending_since = None;
                                    state.retry_after = Some(
                                        Instant::now()
                                            + Duration::from_millis(state.retry_delay_ms),
                                    );
                                    state.retry_delay_ms =
                                        state.retry_delay_ms.saturating_mul(2).min(60_000);
                                }
                            }
                        }
                    }
                    Ok(Err(err)) => {
                        server_event(
                            "error",
                            "source_reload_rejected",
                            "reload",
                            &format!("domain={key} generation={} error={err}", domain.generation),
                        );
                        server_log(&format!(
                            "{{\"event\":\"reload_previous_generation_retained\",\"domain\":\"{}\",\"generation\":{},\"rejected_candidate_generation\":{}}}",
                            json_log_escape(&key),
                            domain.generation,
                            domain.generation.saturating_add(1)
                        ));
                        if let Some(rustc) = err.rustc_diagnostics() {
                            server_event(
                                "error",
                                "source_reload_rustc_diagnostics",
                                "compiler",
                                &format!("domain={key} diagnostic={rustc}"),
                            );
                        }
                        if domain.reload.debug_compile_errors {
                            let diagnostic = if let Some(compile_error) = err.compiler_error() {
                                compile_error.render_debug(&domain.app)
                            } else if let Some(rustc) = err.rustc_diagnostics() {
                                format!("error[RUSTC]: native cdylib compilation failed\n{rustc}")
                            } else {
                                format!("error[RELOAD]: {err}")
                            };
                            eprintln!(
                                "\nVelran candidate build rejected; serving last valid generation {}.",
                                domain.generation
                            );
                            eprintln!("{diagnostic}");
                            server_event(
                                "error",
                                "source_reload_diagnostics",
                                "reload",
                                &format!("domain={key} diagnostic={diagnostic}"),
                            );
                            crate::dev_compile_error::set(&key, &diagnostic);
                            eprintln!();
                        }
                        if let Some(state) = states.get_mut(&key) {
                            state.failed_observed = Some(failed_snapshot.clone());
                            state.pending_since = None;
                            state.retry_after =
                                Some(Instant::now() + Duration::from_millis(state.retry_delay_ms));
                            state.retry_delay_ms =
                                state.retry_delay_ms.saturating_mul(2).min(60_000);
                        }
                    }
                    Err(err) => {
                        server_event(
                            "error",
                            "source_reload_task_failed",
                            "reload",
                            &format!("domain={key} error={err}"),
                        );
                        if let Some(state) = states.get_mut(&key) {
                            state.failed_observed = Some(failed_snapshot.clone());
                            state.pending_since = None;
                            state.retry_after =
                                Some(Instant::now() + Duration::from_millis(state.retry_delay_ms));
                            state.retry_delay_ms =
                                state.retry_delay_ms.saturating_mul(2).min(60_000);
                        }
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn source_snapshot_detects_same_length_content_changes() {
        let path = std::env::temp_dir().join(format!(
            "velran-rolling-reload-fingerprint-{}-{}.vrn",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        {
            let mut file = fs::File::create(&path).unwrap();
            file.write_all(b"let a = 1;\n").unwrap();
        }
        let first = snapshot_source_files(std::slice::from_ref(&path));
        {
            let mut file = fs::File::create(&path).unwrap();
            file.write_all(b"let b = 2;\n").unwrap();
        }
        let second = snapshot_source_files(std::slice::from_ref(&path));
        assert_eq!(first[0].len, second[0].len);
        assert_ne!(first[0].sha256, second[0].sha256);
        assert_ne!(first, second);
        let _ = fs::remove_file(path);
    }
}
