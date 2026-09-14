use crate::auth_setup::canonical_username;
use crate::server_errors::AuthSetupError;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub(super) fn load_memberships(
    path: Option<&Path>,
) -> Result<HashMap<String, Vec<auth::TenantId>>, AuthSetupError> {
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
                message: "expected username=tenant,tenant",
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
        let mut memberships = Vec::new();
        for tenant in raw
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let parsed =
                auth::TenantId::parse(tenant).map_err(|_| AuthSetupError::InvalidTenant {
                    path: path.to_path_buf(),
                    line: line_no,
                    tenant: tenant.to_string(),
                })?;
            memberships.push(parsed);
        }
        auth::validate_memberships(&memberships).map_err(|_| AuthSetupError::InvalidLine {
            path: path.to_path_buf(),
            line: line_no,
            message: "tenant list must contain at most 64 unique tenant IDs",
        })?;
        out.insert(user, memberships);
    }
    Ok(out)
}
