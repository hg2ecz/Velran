#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Webhook {
    pub name: String,
    pub secret_name: String,
    pub signature_header: String,
    pub timestamp_header: String,
    pub replay_window_secs: u64,
}
