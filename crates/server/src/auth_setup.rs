use crate::server_errors::AuthSetupError;
use crate::{AuthCliConfig, LdapConfig};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

pub(super) fn build_ldap_config(c: &AuthCliConfig) -> Result<Option<LdapConfig>, AuthSetupError> {
    let any = c.ldap_url.is_some()
        || c.ldap_search_base.is_some()
        || c.ldap_bind_dn.is_some()
        || c.ldap_bind_password.is_some();
    if !any {
        return Ok(None);
    }

    let cfg = LdapConfig {
        url: c
            .ldap_url
            .clone()
            .ok_or(AuthSetupError::MissingLdapField("--ldap-url"))?,
        search_base: c
            .ldap_search_base
            .clone()
            .ok_or(AuthSetupError::MissingLdapField("--ldap-search-base"))?,
        username_attribute: c.ldap_username_attribute.clone(),
        service_bind_dn: c
            .ldap_bind_dn
            .clone()
            .ok_or(AuthSetupError::MissingLdapField("bind DN file"))?,
        service_bind_password: c
            .ldap_bind_password
            .clone()
            .ok_or(AuthSetupError::MissingLdapField("bind password file"))?,
        timeout: Duration::from_secs(5),
    };
    cfg.validate().map_err(AuthSetupError::from)?;
    Ok(Some(cfg))
}

pub(super) fn load_totp_secrets(
    path: Option<&Path>,
) -> Result<HashMap<String, Vec<u8>>, AuthSetupError> {
    let Some(path) = path else {
        return Ok(HashMap::new());
    };
    let text = fs::read_to_string(path).map_err(|source| AuthSetupError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    let mut out = HashMap::new();
    for (raw_no, line) in text.lines().enumerate() {
        let line_no = raw_no + 1;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (user, hex) = line
            .split_once('=')
            .ok_or_else(|| AuthSetupError::InvalidLine {
                path: path.to_path_buf(),
                line: line_no,
                message: "expected username=hexsecret",
            })?;
        let user = canonical_username(user).ok_or_else(|| AuthSetupError::InvalidUsername {
            path: path.to_path_buf(),
            line: line_no,
        })?;
        if out.contains_key(&user) {
            return Err(AuthSetupError::DuplicateUsername {
                path: path.to_path_buf(),
                line: line_no,
            });
        }
        let bytes = decode_hex(hex).ok_or_else(|| AuthSetupError::InvalidTotpHex {
            path: path.to_path_buf(),
            line: line_no,
        })?;
        if bytes.len() < 20 {
            return Err(AuthSetupError::TotpSecretTooShort {
                path: path.to_path_buf(),
                line: line_no,
            });
        }
        out.insert(user, bytes);
    }
    Ok(out)
}

pub(super) fn decode_hex(v: &str) -> Option<Vec<u8>> {
    if v.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::new();
    for pair in v.as_bytes().chunks_exact(2) {
        let h = (pair[0] as char).to_digit(16)?;
        let l = (pair[1] as char).to_digit(16)?;
        out.push(((h << 4) | l) as u8);
    }
    Some(out)
}

pub(super) fn load_roles(
    path: Option<&Path>,
) -> Result<HashMap<String, Vec<String>>, AuthSetupError> {
    let Some(path) = path else {
        return Ok(HashMap::new());
    };
    let text = fs::read_to_string(path).map_err(|source| AuthSetupError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    let mut out = HashMap::new();
    for (raw_no, line) in text.lines().enumerate() {
        let line_no = raw_no + 1;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (user, raw) = line
            .split_once('=')
            .ok_or_else(|| AuthSetupError::InvalidLine {
                path: path.to_path_buf(),
                line: line_no,
                message: "expected username=Role,Role",
            })?;
        let user = canonical_username(user).ok_or_else(|| AuthSetupError::InvalidUsername {
            path: path.to_path_buf(),
            line: line_no,
        })?;
        let mut roles = Vec::new();
        for role in raw.split(',').map(str::trim).filter(|v| !v.is_empty()) {
            if !role.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
                return Err(AuthSetupError::InvalidRole {
                    path: path.to_path_buf(),
                    line: line_no,
                    role: role.to_string(),
                });
            }
            roles.push(role.into());
        }
        out.insert(user, roles);
    }
    Ok(out)
}

pub(super) fn canonical_username(raw: &str) -> Option<String> {
    let v = raw.trim();
    if v.is_empty()
        || v.len() > 128
        || !v.is_ascii()
        || !v
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b'@'))
    {
        return None;
    }
    Some(v.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn totp_loader_reports_typed_line_error() {
        let path = std::env::temp_dir().join(format!("velran-totp-{}.txt", std::process::id()));
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "bad line").unwrap();
        let error = load_totp_secrets(Some(&path)).unwrap_err();
        let _ = std::fs::remove_file(&path);
        assert!(matches!(error, AuthSetupError::InvalidLine { line: 1, .. }));
    }

    #[test]
    fn canonical_username_normalizes_ascii_case() {
        assert_eq!(
            canonical_username("Alice.Example"),
            Some("alice.example".into())
        );
    }
}
