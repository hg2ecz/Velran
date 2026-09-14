use crate::TlsCliConfig;
use crate::server_config_file::DomainCliConfig;
use crate::server_errors::{PublicHostError, TlsConfigError};
use rustls::ServerConfig as RustlsServerConfig;
use rustls::server::ResolvesServerCertUsingSni;
use rustls::sign::CertifiedKey;
use rustls_pemfile::{certs, private_key};
use std::fs;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;

pub(super) fn validate_public_host(raw: &str) -> Result<String, PublicHostError> {
    let v = raw.trim();
    if v.is_empty()
        || v.len() > 253
        || v.contains('/')
        || v.contains('\\')
        || v.contains('@')
        || v.contains('#')
        || v.contains('?')
        || v.bytes().any(|b| b <= 0x20 || b >= 0x7f)
    {
        return Err(PublicHostError::new(raw));
    }
    let host = v.strip_suffix('.').unwrap_or(v);
    if host.is_empty()
        || !host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
    {
        return Err(PublicHostError::new(raw));
    }
    Ok(host.to_ascii_lowercase())
}

pub(super) fn request_public_host(raw: Option<&str>) -> Option<String> {
    let raw = raw?;
    let value = raw.trim();
    if value.is_empty() || value.contains(',') || value.bytes().any(|b| b <= 0x20 || b >= 0x7f) {
        return None;
    }
    let host = if let Some((host, port)) = value.rsplit_once(':') {
        if host.contains(':') || port.is_empty() || port.parse::<u16>().is_err() {
            return None;
        }
        host
    } else {
        value
    };
    let host = host.strip_suffix('.').unwrap_or(host);
    validate_public_host(host).ok()
}

pub(super) fn host_matches_public(raw: Option<&str>, expected: &str) -> bool {
    request_public_host(raw).as_deref() == Some(expected)
}

fn read_certificate_chain(
    cert_path: &Path,
) -> Result<Vec<rustls::pki_types::CertificateDer<'static>>, TlsConfigError> {
    let file = fs::File::open(cert_path)
        .map_err(|e| TlsConfigError::io("open TLS certificate file", cert_path, e))?;
    let mut reader = BufReader::new(file);
    let cert_chain = certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| TlsConfigError::io("read TLS certificate file", cert_path, e))?;
    if cert_chain.is_empty() {
        return Err(TlsConfigError::invalid(format!(
            "TLS certificate file `{}` contains no certificates",
            cert_path.display()
        )));
    }
    Ok(cert_chain)
}

fn read_private_key(
    key_path: &Path,
) -> Result<rustls::pki_types::PrivateKeyDer<'static>, TlsConfigError> {
    let file = fs::File::open(key_path)
        .map_err(|e| TlsConfigError::io("open TLS private key file", key_path, e))?;
    let mut reader = BufReader::new(file);
    private_key(&mut reader)
        .map_err(|e| TlsConfigError::io("read TLS private key file", key_path, e))?
        .ok_or_else(|| {
            TlsConfigError::invalid(format!(
                "TLS private key file `{}` contains no supported private key",
                key_path.display()
            ))
        })
}

pub(super) fn load_certified_key(
    cert_path: &Path,
    key_path: &Path,
    provider: &rustls::crypto::CryptoProvider,
) -> Result<CertifiedKey, TlsConfigError> {
    let cert_chain = read_certificate_chain(cert_path)?;
    let key = read_private_key(key_path)?;
    CertifiedKey::from_der(cert_chain, key, provider).map_err(TlsConfigError::rustls)
}

pub(super) fn build_tls_acceptor(
    c: &TlsCliConfig,
    domains: &[DomainCliConfig],
) -> Result<Option<TlsAcceptor>, TlsConfigError> {
    if domains.is_empty() {
        let (Some(cert_path), Some(key_path)) = (&c.cert_file, &c.key_file) else {
            return Ok(None);
        };
        let cert_chain = read_certificate_chain(cert_path)?;
        let key = read_private_key(key_path)?;
        let mut cfg = RustlsServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, key)
            .map_err(TlsConfigError::rustls)?;
        cfg.alpn_protocols = vec![b"http/1.1".to_vec()];
        return Ok(Some(TlsAcceptor::from(Arc::new(cfg))));
    }

    let global_pair = match (&c.cert_file, &c.key_file) {
        (Some(cert), Some(key)) => Some((cert.as_path(), key.as_path())),
        _ => None,
    };
    let any_domain_tls = domains.iter().any(|d| d.tls.is_some());
    if global_pair.is_none() && !any_domain_tls {
        return Ok(None);
    }

    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let mut resolver = ResolvesServerCertUsingSni::new();
    for domain in domains {
        let (cert_path, key_path) = match domain.tls.as_ref() {
            Some(tls) => (tls.cert_file.as_path(), tls.key_file.as_path()),
            None => global_pair.ok_or_else(|| {
                TlsConfigError::invalid(format!(
                    "TLS is enabled, but domain `{}` has no [domains.tls] certificate and no global [tls] fallback",
                    domain.host
                ))
            })?,
        };
        let certified_key = load_certified_key(cert_path, key_path, provider.as_ref())?;
        for name in std::iter::once(&domain.host).chain(domain.aliases.iter()) {
            resolver.add(name, certified_key.clone()).map_err(|e| {
                TlsConfigError::invalid(format!(
                    "TLS certificate `{}` is not valid for configured host/alias `{name}`: {e}",
                    cert_path.display()
                ))
            })?;
        }
    }
    let mut cfg = RustlsServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(TlsConfigError::rustls)?
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(resolver));
    cfg.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(Some(TlsAcceptor::from(Arc::new(cfg))))
}
