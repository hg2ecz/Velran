use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct EgressConfig {
    #[serde(default)]
    pub target: Vec<TargetConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TargetConfig {
    pub name: String,
    pub hosts: Vec<String>,
    pub cidrs: Vec<String>,
    #[serde(default = "default_ports")]
    pub ports: Vec<u16>,
    #[serde(default = "default_true")]
    pub tls_required: bool,
    #[serde(default = "default_dns_answers")]
    pub max_dns_answers: usize,
    #[serde(default = "default_send")]
    pub max_sent_bytes: usize,
    #[serde(default = "default_recv")]
    pub max_received_bytes: usize,
    #[serde(default = "default_connect_ms")]
    pub connect_timeout_ms: u64,
    #[serde(default = "default_total_ms")]
    pub total_timeout_ms: u64,
}

fn default_ports() -> Vec<u16> {
    vec![443]
}
fn default_true() -> bool {
    true
}
fn default_dns_answers() -> usize {
    16
}
fn default_send() -> usize {
    256 * 1024
}
fn default_recv() -> usize {
    2 * 1024 * 1024
}
fn default_connect_ms() -> u64 {
    5_000
}
fn default_total_ms() -> u64 {
    15_000
}
