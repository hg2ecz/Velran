use crate::error::ObsError;
use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct SystemEvent<'a> {
    pub schema_version: u8,
    pub timestamp: String,
    pub event: &'a str,
    pub level: &'a str,
    pub component: &'a str,
    pub message: &'a str,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestLog<'a> {
    pub schema_version: u8,
    pub timestamp: String,
    pub event: &'static str,
    pub request_id: &'a str,
    pub method: &'a str,
    pub route: &'a str,
    pub status: u16,
    pub duration_ms: u128,
    pub client_ip: &'a str,
    pub bytes_in: u64,
    pub bytes_out: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent<'a> {
    pub schema_version: u8,
    pub timestamp: String,
    pub event: &'static str,
    pub request_id: &'a str,
    pub category: &'a str,
    pub action: &'a str,
    pub outcome: &'a str,
    pub detail: &'a str,
}

#[derive(Debug, Clone, Serialize)]
pub struct SecurityEvent<'a> {
    pub schema_version: u8,
    pub timestamp: String,
    pub event: &'static str,
    pub level: &'a str,
    pub category: &'a str,
    pub action: &'a str,
    pub outcome: &'a str,
    pub request_id: &'a str,
    pub route: &'a str,
    pub subject: &'a str,
    pub source: &'a str,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActivityEvent<'a> {
    pub schema_version: u8,
    pub timestamp: String,
    pub event: &'static str,
    pub request_id: &'a str,
    pub actor: &'a str,
    pub action: &'a str,
    pub target: &'a str,
    pub outcome: &'a str,
    pub client_ip: &'a str,
}

pub fn utc_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn json_line<T: Serialize>(value: &T) -> Result<String, ObsError> {
    serde_json::to_string(value).map_err(|_| ObsError::Serialization)
}

static REQUEST_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
pub fn new_request_id() -> String {
    let counter = REQUEST_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("velran-{nanos:032x}-{counter:016x}")
}

#[cfg(test)]
mod tests {
    use super::{
        ActivityEvent, RequestLog, SecurityEvent, json_line, new_request_id, utc_timestamp,
    };

    #[test]
    fn request_id_is_server_generated_and_unique() {
        assert_ne!(new_request_id(), new_request_id());
    }

    #[test]
    fn secret_free_structured_log_shape() {
        let line = json_line(&RequestLog {
            schema_version: 1,
            timestamp: utc_timestamp(),
            event: "http_request",
            request_id: "req_test",
            method: "GET",
            route: "home",
            status: 200,
            duration_ms: 3,
            client_ip: "127.0.0.1",
            bytes_in: 0,
            bytes_out: 12,
        })
        .unwrap();
        assert!(line.contains("req_test"));
        assert!(!line.contains("authorization"));
        assert!(!line.contains("cookie"));
    }

    #[test]
    fn activity_has_utc_timestamp_and_no_payload_field() {
        let line = json_line(&ActivityEvent {
            schema_version: 1,
            timestamp: utc_timestamp(),
            event: "user_activity",
            request_id: "velran-test",
            actor: "alice",
            action: "articlePublish",
            target: "application_route",
            outcome: "success",
            client_ip: "127.0.0.1",
        })
        .unwrap();
        assert!(line.contains("\"schema_version\":1"));
        assert!(line.contains("Z\""));
        assert!(line.contains("articlePublish"));
        assert!(!line.contains("password"));
        assert!(!line.contains("request_body"));
    }
    #[test]
    fn security_event_schema_is_structured_and_redacted() {
        let line = json_line(&SecurityEvent {
            schema_version: 1,
            timestamp: utc_timestamp(),
            event: "security_event",
            level: "warn",
            category: "auth",
            action: "authenticate",
            outcome: "invalid_credentials",
            request_id: "velran-test",
            route: "/__velran/auth/login",
            subject: "[redacted]",
            source: "[redacted]",
        })
        .unwrap();
        assert!(line.contains("\"subject\":\"[redacted]\""));
        assert!(line.contains("\"source\":\"[redacted]\""));
        assert!(!line.contains("alice@example.com"));
        assert!(!line.contains("127.0.0.1"));
    }
}
