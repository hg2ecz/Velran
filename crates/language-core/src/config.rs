use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub listen: SocketAddr,
    pub max_header_bytes: usize,
    pub max_body_bytes: usize,
    pub max_connections: usize,
    pub max_requests_per_connection: usize,
    pub read_timeout_ms: u64,
    pub request_timeout_ms: u64,
    pub write_timeout_ms: u64,
    pub max_header_count: usize,
    pub max_form_fields: usize,
    pub max_form_field_bytes: usize,
    pub max_json_depth: usize,
    pub max_json_string_bytes: usize,
    pub max_json_array_items: usize,
    pub max_json_object_fields: usize,
    pub max_instructions: u64,
    pub max_runtime_alloc_bytes: u64,
    pub session_ttl_secs: u64,
    pub max_sessions: usize,
    pub insecure_dev_cookies: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen: "127.0.0.1:8080"
                .parse()
                .expect("valid default listen address"),
            max_header_bytes: 16 * 1024,
            max_body_bytes: 256 * 1024,
            max_connections: 4096,
            max_requests_per_connection: 100,
            read_timeout_ms: 5_000,
            request_timeout_ms: 15_000,
            write_timeout_ms: 5_000,
            max_header_count: 64,
            max_form_fields: 64,
            max_form_field_bytes: 8 * 1024,
            max_json_depth: 32,
            max_json_string_bytes: 64 * 1024,
            max_json_array_items: 4096,
            max_json_object_fields: 1024,
            max_instructions: 100_000,
            max_runtime_alloc_bytes: 32 * 1024 * 1024,
            session_ttl_secs: 8 * 60 * 60,
            max_sessions: 100_000,
            insecure_dev_cookies: false,
        }
    }
}
