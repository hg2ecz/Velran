use crate::error::IntegrationError;
use ipnet::IpNet;
use std::collections::HashMap;
use std::fs;
use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

const MAX_DNS_ANSWERS_HARD: usize = 64;

pub use crate::egress_config::{EgressConfig, TargetConfig};

#[derive(Clone)]
pub struct EgressPolicy {
    targets: Arc<HashMap<String, Target>>,
}

#[derive(Clone)]
pub(crate) struct Target {
    pub(crate) hosts: Vec<String>,
    pub(crate) cidrs: Vec<IpNet>,
    pub(crate) ports: Vec<u16>,
    pub(crate) tls_required: bool,
    pub(crate) max_dns_answers: usize,
    pub(crate) max_sent_bytes: usize,
    pub(crate) max_received_bytes: usize,
    pub(crate) connect_timeout: Duration,
    pub(crate) total_timeout: Duration,
}

impl EgressPolicy {
    pub fn from_toml_file(path: impl AsRef<Path>) -> Result<Self, IntegrationError> {
        let bytes =
            fs::read(path).map_err(|_| IntegrationError::Policy("cannot read policy".into()))?;
        if bytes.len() > 1024 * 1024 {
            return Err(IntegrationError::Policy("policy too large".into()));
        }
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| IntegrationError::Policy("policy is not UTF-8".into()))?;
        let cfg: EgressConfig =
            toml::from_str(text).map_err(|_| IntegrationError::Policy("invalid TOML".into()))?;
        Self::from_config(cfg)
    }

    pub fn from_config(cfg: EgressConfig) -> Result<Self, IntegrationError> {
        let mut targets = HashMap::new();
        for c in cfg.target {
            validate_name(&c.name)?;
            if c.hosts.is_empty() || c.cidrs.is_empty() || c.ports.is_empty() {
                return Err(IntegrationError::Policy(format!(
                    "target {} is incomplete",
                    c.name
                )));
            }
            if !c.tls_required {
                return Err(IntegrationError::Policy(format!(
                    "target {} must require TLS in M14",
                    c.name
                )));
            }
            if c.max_dns_answers == 0 || c.max_dns_answers > MAX_DNS_ANSWERS_HARD {
                return Err(IntegrationError::Policy("invalid DNS answer limit".into()));
            }
            let mut hosts = Vec::new();
            for h in c.hosts {
                hosts.push(canonical_host(&h)?);
            }
            let mut cidrs = Vec::new();
            for n in c.cidrs {
                cidrs.push(
                    n.parse()
                        .map_err(|_| IntegrationError::Policy("invalid CIDR".into()))?,
                );
            }
            if targets.contains_key(&c.name) {
                return Err(IntegrationError::Policy("duplicate target name".into()));
            }
            targets.insert(
                c.name.clone(),
                Target {
                    hosts,
                    cidrs,
                    ports: c.ports,
                    tls_required: c.tls_required,
                    max_dns_answers: c.max_dns_answers,
                    max_sent_bytes: c.max_sent_bytes,
                    max_received_bytes: c.max_received_bytes,
                    connect_timeout: Duration::from_millis(c.connect_timeout_ms),
                    total_timeout: Duration::from_millis(c.total_timeout_ms),
                },
            );
        }
        Ok(Self {
            targets: Arc::new(targets),
        })
    }

    pub(crate) fn target(&self, name: &str) -> Result<&Target, IntegrationError> {
        self.targets
            .get(name)
            .ok_or_else(|| IntegrationError::Policy("unknown target".into()))
    }

    pub fn capability(
        &self,
        name: &str,
    ) -> Result<crate::egress_capability::EgressCapability, IntegrationError> {
        Ok(crate::egress_capability::EgressCapability::new(
            self.target(name)?.clone(),
        ))
    }
}

fn validate_name(v: &str) -> Result<(), IntegrationError> {
    if v.is_empty()
        || v.len() > 64
        || !v
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(IntegrationError::Policy("invalid target name".into()));
    }
    Ok(())
}

pub(crate) fn policy_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6
            .to_ipv4_mapped()
            .map(IpAddr::V4)
            .unwrap_or(IpAddr::V6(v6)),
        v4 => v4,
    }
}

pub(crate) fn ip_allowed(cidrs: &[IpNet], ip: IpAddr) -> bool {
    let ip = policy_ip(ip);
    cidrs.iter().any(|net| match (net, ip) {
        (IpNet::V4(net), IpAddr::V4(ip)) => net.contains(&ip),
        (IpNet::V6(net), IpAddr::V6(ip)) => net.contains(&ip),
        _ => false,
    })
}

pub(crate) fn canonical_host(v: &str) -> Result<String, IntegrationError> {
    let h = v.trim_end_matches('.').to_ascii_lowercase();
    if h.is_empty()
        || h.len() > 253
        || h.parse::<IpAddr>().is_ok()
        || !h
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
    {
        return Err(IntegrationError::Policy("invalid hostname".into()));
    }
    if h.split('.')
        .any(|p| p.is_empty() || p.len() > 63 || p.starts_with('-') || p.ends_with('-'))
    {
        return Err(IntegrationError::Policy("invalid hostname".into()));
    }
    Ok(h)
}

#[cfg(test)]
mod tests {
    use super::{EgressConfig, EgressPolicy, TargetConfig, canonical_host, ip_allowed, policy_ip};
    use ipnet::IpNet;
    use std::net::IpAddr;

    fn policy() -> EgressPolicy {
        EgressPolicy::from_config(EgressConfig {
            target: vec![TargetConfig {
                name: "payments".into(),
                hosts: vec!["api.example.com".into()],
                cidrs: vec!["203.0.113.0/24".into()],
                ports: vec![443],
                tls_required: true,
                max_dns_answers: 8,
                max_sent_bytes: 1024,
                max_received_bytes: 4096,
                connect_timeout_ms: 1000,
                total_timeout_ms: 2000,
            }],
        })
        .unwrap()
    }

    #[test]
    fn rejects_target_rule_mixing() {
        let p = policy();
        assert!(p.target("payments").is_ok());
        assert!(canonical_host("127.0.0.1").is_err());
    }

    #[test]
    fn ipv6_policy_accepts_global_address_in_explicit_cidr() {
        let cidrs = vec!["2001:db8::/32".parse::<IpNet>().unwrap()];
        assert!(ip_allowed(&cidrs, "2001:db8::42".parse().unwrap()));
        assert!(!ip_allowed(&cidrs, "2001:db9::42".parse().unwrap()));
    }

    #[test]
    fn mixed_family_policy_is_fail_closed_per_address() {
        let cidrs = vec![
            "203.0.113.0/24".parse::<IpNet>().unwrap(),
            "2001:db8::/32".parse::<IpNet>().unwrap(),
        ];
        assert!(ip_allowed(&cidrs, "203.0.113.7".parse().unwrap()));
        assert!(ip_allowed(&cidrs, "2001:db8::7".parse().unwrap()));
        assert!(!ip_allowed(&cidrs, "2001:db9::7".parse().unwrap()));
    }

    #[test]
    fn ipv4_mapped_ipv6_uses_ipv4_policy() {
        let v4 = vec!["203.0.113.0/24".parse::<IpNet>().unwrap()];
        let mapped: IpAddr = "::ffff:203.0.113.9".parse().unwrap();
        assert_eq!(
            policy_ip(mapped),
            IpAddr::V4(std::net::Ipv4Addr::new(203, 0, 113, 9))
        );
        assert!(ip_allowed(&v4, mapped));
        let mapped_v6_only = vec!["::ffff:0:0/96".parse::<IpNet>().unwrap()];
        assert!(!ip_allowed(&mapped_v6_only, mapped));
    }

    #[test]
    fn special_ipv6_ranges_require_explicit_cidr() {
        let global = vec!["2001:db8::/32".parse::<IpNet>().unwrap()];
        for ip in ["::1", "fe80::1", "fc00::1", "ff02::1", "::"] {
            assert!(!ip_allowed(&global, ip.parse().unwrap()), "{ip}");
        }
        assert!(ip_allowed(
            &["fe80::/10".parse::<IpNet>().unwrap()],
            "fe80::1".parse().unwrap()
        ));
    }

    #[test]
    fn capability_binds_endpoint_to_named_policy() {
        let capability = policy().capability("payments").unwrap();
        assert!(capability.endpoint("api.example.com", 443).is_ok());
        assert!(capability.endpoint("evil.example.com", 443).is_err());
        assert!(capability.endpoint("api.example.com", 8443).is_err());
    }

    #[test]
    fn target_requires_tls() {
        let x = EgressPolicy::from_config(EgressConfig {
            target: vec![TargetConfig {
                name: "x".into(),
                hosts: vec!["a.example".into()],
                cidrs: vec!["10.0.0.0/8".into()],
                ports: vec![80],
                tls_required: false,
                max_dns_answers: 1,
                max_sent_bytes: 1,
                max_received_bytes: 1,
                connect_timeout_ms: 1,
                total_timeout_ms: 1,
            }],
        });
        assert!(x.is_err());
    }
}
