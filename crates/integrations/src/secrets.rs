use crate::error::IntegrationError;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

const MAX_SECRET_BYTES: usize = 64 * 1024;

pub struct SecretString(Vec<u8>);

impl SecretString {
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Drop for SecretString {
    fn drop(&mut self) {
        self.0.fill(0);
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString([REDACTED])")
    }
}

#[derive(Clone)]
pub struct SecretsStore {
    root: Arc<PathBuf>,
}

impl SecretsStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, IntegrationError> {
        let root = root.into();
        let meta = fs::symlink_metadata(&root)
            .map_err(|_| IntegrationError::Secret("secret root unavailable".into()))?;
        if !meta.file_type().is_dir() || meta.file_type().is_symlink() {
            return Err(IntegrationError::Secret("invalid secret root".into()));
        }
        Ok(Self {
            root: Arc::new(root),
        })
    }

    pub fn get(&self, name: &str) -> Result<SecretString, IntegrationError> {
        validate_secret_name(name)?;
        let path = self.root.join(name);
        let mut opts = OpenOptions::new();
        opts.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
        }
        let file: File = opts
            .open(&path)
            .map_err(|_| IntegrationError::Secret("secret unavailable".into()))?;
        let meta = file
            .metadata()
            .map_err(|_| IntegrationError::Secret("secret metadata unavailable".into()))?;
        if !meta.is_file() || meta.len() as usize > MAX_SECRET_BYTES {
            return Err(IntegrationError::Secret("invalid secret file".into()));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if meta.mode() & 0o007 != 0 {
                return Err(IntegrationError::Secret(
                    "secret is accessible by others".into(),
                ));
            }
        }
        let mut value = Vec::with_capacity(meta.len() as usize);
        file.take((MAX_SECRET_BYTES + 1) as u64)
            .read_to_end(&mut value)
            .map_err(|_| IntegrationError::Secret("secret read failed".into()))?;
        if value.len() > MAX_SECRET_BYTES {
            return Err(IntegrationError::Secret("secret too large".into()));
        }
        while matches!(value.last(), Some(b'\n' | b'\r')) {
            value.pop();
        }
        if value.is_empty() {
            return Err(IntegrationError::Secret("secret is empty".into()));
        }
        Ok(SecretString(value))
    }
}

fn validate_secret_name(v: &str) -> Result<(), IntegrationError> {
    if v.is_empty()
        || v.len() > 128
        || v == "."
        || v == ".."
        || v.contains('/')
        || v.contains('\\')
        || !v
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(IntegrationError::Secret("invalid secret name".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_secret_name;

    #[test]
    fn secret_name_is_confined() {
        assert!(validate_secret_name("api-key").is_ok());
        assert!(validate_secret_name("../api-key").is_err());
        assert!(validate_secret_name("x/y").is_err());
    }
}
