use observability::security_event;
use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_secs(300);
const ALERT_COOLDOWN: Duration = Duration::from_secs(300);

#[derive(Default)]
struct Bucket {
    events: VecDeque<Instant>,
    last_alert: Option<Instant>,
}

static BUCKETS: OnceLock<Mutex<HashMap<String, Bucket>>> = OnceLock::new();

pub(super) fn observe(
    category: &str,
    action: &str,
    outcome: &str,
    correlation_key: &str,
    request_id: &str,
    route: &str,
) {
    security_event("warn", category, action, outcome, request_id, route);
    let threshold = threshold_for(category, outcome);
    let now = Instant::now();
    let buckets = BUCKETS.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut buckets) = buckets.lock() else {
        return;
    };
    let key = format!("{category}\n{outcome}\n{correlation_key}");
    let bucket = buckets.entry(key).or_default();
    while bucket
        .events
        .front()
        .is_some_and(|seen| now.duration_since(*seen) > WINDOW)
    {
        bucket.events.pop_front();
    }
    bucket.events.push_back(now);
    let cooled_down = bucket
        .last_alert
        .map_or(true, |last| now.duration_since(last) >= ALERT_COOLDOWN);
    if bucket.events.len() >= threshold && cooled_down {
        bucket.last_alert = Some(now);
        security_event(
            "error",
            category,
            "threshold_exceeded",
            "alert",
            request_id,
            route,
        );
    }
}

fn threshold_for(category: &str, outcome: &str) -> usize {
    match (category, outcome) {
        ("auth", _) => 12,
        ("mfa", _) => 8,
        ("webhook", _) => 8,
        ("idempotency", _) => 8,
        ("rate_limit", _) => 5,
        ("resource", _) => 8,
        ("policy", _) | ("csrf", _) | ("origin", _) | ("cors", _) => 20,
        _ => 25,
    }
}

#[cfg(test)]
mod tests {
    use super::threshold_for;

    #[test]
    fn sensitive_security_categories_have_tighter_alert_thresholds() {
        assert!(threshold_for("webhook", "denied") < threshold_for("policy", "denied"));
        assert!(threshold_for("rate_limit", "denied") < threshold_for("auth", "unauthorized"));
    }
}
