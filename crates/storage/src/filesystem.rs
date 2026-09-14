use crate::upload::UploadResult;
use rustix::fd::OwnedFd;
use rustix::fs::{
    AtFlags, FileType, Mode, OFlags, RenameFlags, ResolveFlags, fstat, open, openat2,
    renameat_with, unlinkat,
};
use std::fs::File;
use std::path::{Component, Path};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug)]
pub enum FsError {
    Denied,
    InvalidPath,
    NotRegular,
    HardLink,
    FileTooLarge,
    OpenAt2Unavailable,
    Io,
}

impl std::fmt::Display for FsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Denied => "filesystem permission denied",
            Self::InvalidPath => "invalid relative path",
            Self::NotRegular => "filesystem object is not a regular file",
            Self::HardLink => "hard-linked files are not accepted",
            Self::FileTooLarge => "file exceeds configured byte limit",
            Self::OpenAt2Unavailable => "openat2 is required but unavailable",
            Self::Io => "filesystem I/O error",
        };
        f.write_str(message)
    }
}

impl std::error::Error for FsError {}

#[derive(Debug, Clone, Copy)]
pub struct FsMode {
    pub read: bool,
    pub write: bool,
    pub create: bool,
}

impl FsMode {
    pub fn parse(raw: &str) -> Result<Self, FsError> {
        let mut mode = Self {
            read: false,
            write: false,
            create: false,
        };
        if raw.is_empty() {
            return Err(FsError::Denied);
        }
        for ch in raw.chars() {
            match ch {
                'r' if !mode.read => mode.read = true,
                'w' if !mode.write => mode.write = true,
                'c' if !mode.create => mode.create = true,
                _ => return Err(FsError::Denied),
            }
        }
        Ok(mode)
    }
}

#[derive(Debug, Clone)]
pub struct FsLimits {
    pub max_file_bytes: u64,
    pub max_path_depth: usize,
    pub max_component_bytes: usize,
}

impl Default for FsLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: 16 * 1024 * 1024,
            max_path_depth: 16,
            max_component_bytes: 255,
        }
    }
}

#[derive(Clone)]
pub struct AppFs {
    root: Arc<OwnedFd>,
    mode: FsMode,
    limits: FsLimits,
}

impl AppFs {
    pub fn open_root(root: &Path, mode: FsMode, limits: FsLimits) -> Result<Self, FsError> {
        if !root.is_absolute() {
            return Err(FsError::InvalidPath);
        }
        let fd = open(
            root,
            OFlags::PATH | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| FsError::Io)?;
        let stat = fstat(&fd).map_err(|_| FsError::Io)?;
        if !FileType::from_raw_mode(stat.st_mode).is_dir() {
            return Err(FsError::InvalidPath);
        }
        Ok(Self {
            root: Arc::new(fd),
            mode,
            limits,
        })
    }

    pub fn limits(&self) -> &FsLimits {
        &self.limits
    }

    pub fn random_upload_destination(&self, directory: &str) -> Result<String, FsError> {
        self.validate(directory)?;
        Ok(format!("{}/{}.upload", directory, random_hex(16)))
    }

    pub async fn read(&self, relative: &str) -> Result<Vec<u8>, FsError> {
        if !self.mode.read {
            return Err(FsError::Denied);
        }
        self.validate(relative)?;
        let fd = self.open_existing(relative, OFlags::RDONLY | OFlags::CLOEXEC)?;
        let stat = check_regular_single_link(&fd)?;
        if stat.st_size < 0 || stat.st_size as u64 > self.limits.max_file_bytes {
            return Err(FsError::FileTooLarge);
        }
        let file = tokio::fs::File::from_std(File::from(fd));
        let mut out = Vec::with_capacity((stat.st_size as usize).min(64 * 1024));
        file.take(self.limits.max_file_bytes + 1)
            .read_to_end(&mut out)
            .await
            .map_err(|_| FsError::Io)?;
        if out.len() as u64 > self.limits.max_file_bytes {
            return Err(FsError::FileTooLarge);
        }
        Ok(out)
    }

    pub async fn create(&self, relative: &str, bytes: &[u8]) -> Result<(), FsError> {
        if !self.mode.create {
            return Err(FsError::Denied);
        }
        if bytes.len() as u64 > self.limits.max_file_bytes {
            return Err(FsError::FileTooLarge);
        }
        let mut file = self.create_stream(relative).await?;
        file.write_chunk(bytes).await?;
        file.finish().await?;
        Ok(())
    }

    pub async fn create_stream(&self, relative: &str) -> Result<BoundedFile, FsError> {
        if !self.mode.create {
            return Err(FsError::Denied);
        }
        self.validate(relative)?;
        let fd = openat2(
            &*self.root,
            relative,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::RUSR | Mode::WUSR,
            resolve_flags(),
        )
        .map_err(map_open_error)?;
        let _ = check_regular_single_link(&fd)?;
        Ok(BoundedFile {
            file: tokio::fs::File::from_std(File::from(fd)),
            written: 0,
            max: self.limits.max_file_bytes,
        })
    }

    pub async fn overwrite(&self, relative: &str, bytes: &[u8]) -> Result<(), FsError> {
        if !self.mode.write {
            return Err(FsError::Denied);
        }
        if bytes.len() as u64 > self.limits.max_file_bytes {
            return Err(FsError::FileTooLarge);
        }
        self.validate(relative)?;
        let fd = self.open_existing(relative, OFlags::WRONLY | OFlags::TRUNC | OFlags::CLOEXEC)?;
        let _ = check_regular_single_link(&fd)?;
        let mut file = tokio::fs::File::from_std(File::from(fd));
        file.write_all(bytes).await.map_err(|_| FsError::Io)?;
        file.flush().await.map_err(|_| FsError::Io)?;
        Ok(())
    }

    pub fn remove(&self, relative: &str) -> Result<(), FsError> {
        if !self.mode.write {
            return Err(FsError::Denied);
        }
        self.validate(relative)?;
        let fd = self.open_existing(relative, OFlags::RDONLY | OFlags::CLOEXEC)?;
        let _ = check_regular_single_link(&fd)?;
        drop(fd);
        unlinkat(&*self.root, relative, AtFlags::empty()).map_err(|_| FsError::Io)
    }

    fn open_existing(&self, relative: &str, flags: OFlags) -> Result<OwnedFd, FsError> {
        openat2(
            &*self.root,
            relative,
            flags | OFlags::NOFOLLOW,
            Mode::empty(),
            resolve_flags(),
        )
        .map_err(map_open_error)
    }

    pub(crate) async fn create_staged(
        &self,
        destination: &str,
    ) -> Result<(String, BoundedFile), FsError> {
        self.validate(destination)?;
        let parent = Path::new(destination)
            .parent()
            .and_then(Path::to_str)
            .unwrap_or("");
        for _ in 0..8 {
            let suffix = random_hex(16);
            let staging = if parent.is_empty() {
                format!(".vrn-upload-{suffix}.part")
            } else {
                format!("{parent}/.vrn-upload-{suffix}.part")
            };
            let opened = openat2(
                &*self.root,
                &staging,
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC | OFlags::NOFOLLOW,
                Mode::RUSR | Mode::WUSR,
                resolve_flags(),
            );
            match opened {
                Ok(fd) => {
                    let _ = check_regular_single_link(&fd)?;
                    return Ok((
                        staging,
                        BoundedFile {
                            file: tokio::fs::File::from_std(File::from(fd)),
                            written: 0,
                            max: self.limits.max_file_bytes,
                        },
                    ));
                }
                Err(err) if err == rustix::io::Errno::EXIST => continue,
                Err(err) => return Err(map_open_error(err)),
            }
        }
        Err(FsError::Io)
    }

    pub async fn read_staged_upload(&self, upload: &UploadResult) -> Result<Vec<u8>, FsError> {
        self.read(&upload.staging_path).await
    }

    pub fn commit_upload(&self, upload: &UploadResult, destination: &str) -> Result<(), FsError> {
        renameat_with(
            &*self.root,
            &upload.staging_path,
            &*self.root,
            destination,
            RenameFlags::NOREPLACE,
        )
        .map_err(|_| FsError::Denied)
    }

    pub fn cleanup_upload(&self, upload: &UploadResult) {
        self.cleanup_staged(&upload.staging_path);
    }

    pub(crate) fn cleanup_staged(&self, staging: &str) {
        let _ = unlinkat(&*self.root, staging, AtFlags::empty());
    }

    fn validate(&self, relative: &str) -> Result<(), FsError> {
        validate_relative_path(relative, &self.limits)
    }

    pub(crate) fn validate_upload_destination(&self, relative: &str) -> Result<(), FsError> {
        self.validate(relative)
    }

    pub(crate) fn allows_create(&self) -> bool {
        self.mode.create
    }
}

pub struct BoundedFile {
    file: tokio::fs::File,
    written: u64,
    max: u64,
}

impl BoundedFile {
    pub async fn write_chunk(&mut self, bytes: &[u8]) -> Result<(), FsError> {
        let next = self
            .written
            .checked_add(bytes.len() as u64)
            .ok_or(FsError::FileTooLarge)?;
        if next > self.max {
            return Err(FsError::FileTooLarge);
        }
        self.file.write_all(bytes).await.map_err(|_| FsError::Io)?;
        self.written = next;
        Ok(())
    }

    pub async fn finish(mut self) -> Result<u64, FsError> {
        self.file.flush().await.map_err(|_| FsError::Io)?;
        self.file.sync_data().await.map_err(|_| FsError::Io)?;
        Ok(self.written)
    }
}

pub fn validate_relative_path(relative: &str, limits: &FsLimits) -> Result<(), FsError> {
    if relative.is_empty() || relative.len() > 4096 || relative.as_bytes().contains(&0) {
        return Err(FsError::InvalidPath);
    }
    if relative
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(FsError::InvalidPath);
    }
    let path = Path::new(relative);
    if path.is_absolute() {
        return Err(FsError::InvalidPath);
    }
    let mut depth = 0usize;
    for component in path.components() {
        match component {
            Component::Normal(name) => {
                depth += 1;
                if depth > limits.max_path_depth
                    || name.as_encoded_bytes().is_empty()
                    || name.as_encoded_bytes().len() > limits.max_component_bytes
                {
                    return Err(FsError::InvalidPath);
                }
            }
            _ => return Err(FsError::InvalidPath),
        }
    }
    if depth == 0 {
        return Err(FsError::InvalidPath);
    }
    Ok(())
}

fn random_hex(bytes: usize) -> String {
    let mut raw = vec![0u8; bytes];
    rand::fill(&mut raw[..]);
    let mut out = String::with_capacity(bytes * 2);
    for b in raw {
        use std::fmt::Write;
        let _ = write!(out, "{b:02x}");
    }
    out
}

fn resolve_flags() -> ResolveFlags {
    ResolveFlags::BENEATH
        | ResolveFlags::NO_SYMLINKS
        | ResolveFlags::NO_MAGICLINKS
        | ResolveFlags::NO_XDEV
}

fn check_regular_single_link(fd: &OwnedFd) -> Result<rustix::fs::Stat, FsError> {
    let stat = fstat(fd).map_err(|_| FsError::Io)?;
    if !FileType::from_raw_mode(stat.st_mode).is_file() {
        return Err(FsError::NotRegular);
    }
    if stat.st_nlink != 1 {
        return Err(FsError::HardLink);
    }
    Ok(stat)
}

fn map_open_error(err: rustix::io::Errno) -> FsError {
    if err == rustix::io::Errno::NOSYS {
        FsError::OpenAt2Unavailable
    } else {
        FsError::Denied
    }
}

#[cfg(test)]
mod tests {
    use super::{FsLimits, FsMode, validate_relative_path};

    #[test]
    fn rejects_escape_paths() {
        let l = FsLimits::default();
        for bad in ["../x", "/etc/passwd", "a/../b", "./x", "", "a//b"] {
            assert!(validate_relative_path(bad, &l).is_err(), "{bad}");
        }
        assert!(validate_relative_path("uploads/a.bin", &l).is_ok());
    }

    #[test]
    fn parses_permissions_strictly() {
        let m = FsMode::parse("rwc").unwrap();
        assert!(m.read && m.write && m.create);
        assert!(FsMode::parse("rr").is_err());
        assert!(FsMode::parse("x").is_err());
    }
}
