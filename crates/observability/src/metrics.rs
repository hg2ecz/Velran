use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

const LATENCY_BUCKETS_MS: [u64; 8] = [1, 5, 10, 25, 50, 100, 500, 1000];
static LOG_FALLBACK_TOTAL: AtomicU64 = AtomicU64::new(0);

pub(crate) fn increment_log_fallback() {
    LOG_FALLBACK_TOTAL.fetch_add(1, Ordering::Relaxed);
}

#[derive(Default, Clone, Copy)]
struct RouteMetric {
    requests: u64,
    errors_5xx: u64,
    duration_ms_sum: u128,
}

pub struct Metrics {
    requests_total: AtomicU64,
    responses_5xx_total: AtomicU64,
    auth_failures_total: AtomicU64,
    csrf_failures_total: AtomicU64,
    policy_denials_total: AtomicU64,
    request_timeouts_total: AtomicU64,
    runtime_budget_exceeded_total: AtomicU64,
    rate_limit_denials_total: AtomicU64,
    readiness_failures_total: AtomicU64,
    cache_hits_total: AtomicU64,
    cache_misses_total: AtomicU64,
    native_executions_total: AtomicU64,
    bytes_in_total: AtomicU64,
    bytes_out_total: AtomicU64,
    duration_ms_sum: AtomicU64,
    active_connections: AtomicU64,
    latency_buckets: [AtomicU64; 8],
    route_metrics: Mutex<HashMap<String, RouteMetric>>,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            requests_total: AtomicU64::new(0),
            responses_5xx_total: AtomicU64::new(0),
            auth_failures_total: AtomicU64::new(0),
            csrf_failures_total: AtomicU64::new(0),
            policy_denials_total: AtomicU64::new(0),
            request_timeouts_total: AtomicU64::new(0),
            runtime_budget_exceeded_total: AtomicU64::new(0),
            rate_limit_denials_total: AtomicU64::new(0),
            readiness_failures_total: AtomicU64::new(0),
            cache_hits_total: AtomicU64::new(0),
            cache_misses_total: AtomicU64::new(0),
            native_executions_total: AtomicU64::new(0),
            bytes_in_total: AtomicU64::new(0),
            bytes_out_total: AtomicU64::new(0),
            duration_ms_sum: AtomicU64::new(0),
            active_connections: AtomicU64::new(0),
            latency_buckets: std::array::from_fn(|_| AtomicU64::new(0)),
            route_metrics: Mutex::new(HashMap::new()),
        }
    }
}

pub struct ConnectionGuard<'a>(&'a Metrics);
impl Drop for ConnectionGuard<'_> {
    fn drop(&mut self) {
        self.0.active_connections.fetch_sub(1, Ordering::Relaxed);
    }
}

impl Metrics {
    pub fn connection_guard(&self) -> ConnectionGuard<'_> {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        ConnectionGuard(self)
    }
    pub fn record_response(
        &self,
        route: &str,
        status: u16,
        duration: Duration,
        bytes_in: u64,
        bytes_out: u64,
    ) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
        self.bytes_in_total.fetch_add(bytes_in, Ordering::Relaxed);
        self.bytes_out_total.fetch_add(bytes_out, Ordering::Relaxed);
        if status >= 500 {
            self.responses_5xx_total.fetch_add(1, Ordering::Relaxed);
        }
        let ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
        self.duration_ms_sum.fetch_add(ms, Ordering::Relaxed);
        for (i, b) in LATENCY_BUCKETS_MS.iter().enumerate() {
            if ms <= *b {
                self.latency_buckets[i].fetch_add(1, Ordering::Relaxed);
            }
        }
        if let Ok(mut routes) = self.route_metrics.lock() {
            let m = routes.entry(route.to_string()).or_default();
            m.requests = m.requests.saturating_add(1);
            if status >= 500 {
                m.errors_5xx = m.errors_5xx.saturating_add(1);
            }
            m.duration_ms_sum = m.duration_ms_sum.saturating_add(duration.as_millis());
        }
    }
    pub fn inc_auth_failures(&self) {
        self.auth_failures_total.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_csrf_failures(&self) {
        self.csrf_failures_total.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_policy_denials(&self) {
        self.policy_denials_total.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_request_timeouts(&self) {
        self.request_timeouts_total.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_budget_exceeded(&self) {
        self.runtime_budget_exceeded_total
            .fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_rate_limit_denials(&self) {
        self.rate_limit_denials_total
            .fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_readiness_failures(&self) {
        self.readiness_failures_total
            .fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_cache_hit(&self) {
        self.cache_hits_total.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_cache_miss(&self) {
        self.cache_misses_total.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_native_execution(&self) {
        self.native_executions_total.fetch_add(1, Ordering::Relaxed);
    }
    pub fn render_prometheus(&self) -> String {
        let mut out = String::new();
        macro_rules! metric {
            ($name:literal,$help:literal,$v:expr) => {{
                out.push_str(concat!(
                    "# HELP ",
                    $name,
                    " ",
                    $help,
                    "\n# TYPE ",
                    $name,
                    " counter\n",
                    $name,
                    " "
                ));
                out.push_str(&$v.load(Ordering::Relaxed).to_string());
                out.push('\n');
            }};
        }
        metric!(
            "velran_requests_total",
            "Parsed HTTP requests observed by Velran.",
            self.requests_total
        );
        metric!(
            "velran_responses_5xx_total",
            "HTTP 5xx responses.",
            self.responses_5xx_total
        );
        metric!(
            "velran_auth_failures_total",
            "Authentication/authorization failures.",
            self.auth_failures_total
        );
        metric!(
            "velran_csrf_failures_total",
            "CSRF validation failures.",
            self.csrf_failures_total
        );
        metric!(
            "velran_policy_denials_total",
            "Security policy denials.",
            self.policy_denials_total
        );
        metric!(
            "velran_request_timeouts_total",
            "Request/read execution timeouts.",
            self.request_timeouts_total
        );
        metric!(
            "velran_runtime_budget_exceeded_total",
            "Instruction or runtime allocation budget failures.",
            self.runtime_budget_exceeded_total
        );
        metric!(
            "velran_rate_limit_denials_total",
            "Route rate-limit denials.",
            self.rate_limit_denials_total
        );
        metric!(
            "velran_readiness_failures_total",
            "Readiness dependency failures.",
            self.readiness_failures_total
        );
        metric!(
            "velran_cache_hits_total",
            "Public page-cache hits.",
            self.cache_hits_total
        );
        metric!(
            "velran_cache_misses_total",
            "Public page-cache misses.",
            self.cache_misses_total
        );
        metric!(
            "velran_native_executions_total",
            "Requests executed by an activated native shard.",
            self.native_executions_total
        );
        metric!(
            "velran_request_bytes_in_total",
            "Observed request body bytes.",
            self.bytes_in_total
        );
        metric!(
            "velran_response_bytes_out_total",
            "Observed response body bytes.",
            self.bytes_out_total
        );
        out.push_str("# HELP velran_log_fallback_total Log lines that fell back to stderr because the file sink/queue was unavailable.\n# TYPE velran_log_fallback_total counter\nvelran_log_fallback_total ");
        out.push_str(&LOG_FALLBACK_TOTAL.load(Ordering::Relaxed).to_string());
        out.push('\n');
        out.push_str("# HELP velran_active_connections Current accepted connection tasks.\n# TYPE velran_active_connections gauge\nvelran_active_connections ");
        out.push_str(&self.active_connections.load(Ordering::Relaxed).to_string());
        out.push('\n');
        out.push_str("# HELP velran_request_duration_ms Request latency histogram.\n# TYPE velran_request_duration_ms histogram\n");
        for (i, b) in LATENCY_BUCKETS_MS.iter().enumerate() {
            out.push_str(&format!(
                "velran_request_duration_ms_bucket{{le=\"{}\"}} {}\n",
                b,
                self.latency_buckets[i].load(Ordering::Relaxed)
            ));
        }
        out.push_str(&format!(
            "velran_request_duration_ms_bucket{{le=\"+Inf\"}} {}\n",
            self.requests_total.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "velran_request_duration_ms_sum {}\n",
            self.duration_ms_sum.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "velran_request_duration_ms_count {}\n",
            self.requests_total.load(Ordering::Relaxed)
        ));
        if let Ok(routes) = self.route_metrics.lock() {
            out.push_str("# HELP velran_route_requests_total Requests by bounded compile-time route label.\n# TYPE velran_route_requests_total counter\n");
            out.push_str("# HELP velran_route_5xx_total 5xx responses by bounded compile-time route label.\n# TYPE velran_route_5xx_total counter\n");
            out.push_str("# HELP velran_route_duration_ms_sum Accumulated request duration by bounded compile-time route label.\n# TYPE velran_route_duration_ms_sum counter\n");
            let mut items: Vec<_> = routes.iter().collect();
            items.sort_by(|a, b| a.0.cmp(b.0));
            for (route, m) in items {
                let r = prom_label(route);
                out.push_str(&format!(
                    "velran_route_requests_total{{route=\"{}\"}} {}\n",
                    r, m.requests
                ));
                out.push_str(&format!(
                    "velran_route_5xx_total{{route=\"{}\"}} {}\n",
                    r, m.errors_5xx
                ));
                out.push_str(&format!(
                    "velran_route_duration_ms_sum{{route=\"{}\"}} {}\n",
                    r, m.duration_ms_sum
                ));
            }
        }
        out
    }
}
fn prom_label(v: &str) -> String {
    v.chars()
        .flat_map(|c| match c {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect(),
            c if c == '\n' => "\\n".chars().collect(),
            c => vec![c],
        })
        .collect()
}

pub struct RequestTimer(Instant);
impl RequestTimer {
    pub fn start() -> Self {
        Self(Instant::now())
    }
    pub fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::Metrics;
    use std::time::Duration;

    #[test]
    fn prometheus_is_low_cardinality_and_counts() {
        let m = Metrics::default();
        m.record_response("products", 200, Duration::from_millis(7), 10, 20);
        m.record_response("products", 503, Duration::from_millis(80), 0, 5);
        let text = m.render_prometheus();
        assert!(text.contains("velran_requests_total 2"));
        assert!(text.contains("velran_route_requests_total{route=\"products\"} 2"));
        assert!(text.contains("velran_responses_5xx_total 1"));
    }
}
