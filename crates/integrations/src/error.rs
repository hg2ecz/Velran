use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrationError {
    Policy(String),
    Secret(String),
    Dns,
    Connect,
    Tls,
    Timeout,
    Protocol,
    ResponseTooLarge,
    SendTooLarge,
}

impl fmt::Display for IntegrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Policy(v) => write!(f, "egress policy denied: {v}"),
            Self::Secret(v) => write!(f, "secret access denied: {v}"),
            Self::Dns => f.write_str("DNS resolution failed"),
            Self::Connect => f.write_str("outbound connection failed"),
            Self::Tls => f.write_str("TLS verification failed"),
            Self::Timeout => f.write_str("outbound operation timed out"),
            Self::Protocol => f.write_str("invalid upstream HTTP response"),
            Self::ResponseTooLarge => f.write_str("upstream response exceeds policy"),
            Self::SendTooLarge => f.write_str("outbound request exceeds policy"),
        }
    }
}

impl std::error::Error for IntegrationError {}
