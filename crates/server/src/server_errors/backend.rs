use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) enum BackendSupportError {
    Io {
        operation: &'static str,
        subject: String,
        source: io::Error,
    },
    Compiler(compiler::CompileError),
    NativeRuntime(native_runtime::application_runtime::RuntimeError),
    ResourceProfile(super::ResourceProfileConfigError),
    StaticPrefix(super::StaticPrefixError),
    Storage(storage::FsError),
    UnsafeUnixSocketPath(PathBuf),
    #[cfg(not(unix))]
    UnixSocketsUnsupported,
    StaticRouteConflict {
        route: String,
        prefix: String,
    },
    ReservedMediaRoute,
    ReservedHealthRoute {
        path: String,
    },
    HealthStaticConflict {
        path: String,
        prefix: String,
    },
    MissingUploadDataRoot,
    UploadPermissions,
    ImageUploadPermissions,
}

impl BackendSupportError {
    pub(crate) fn io(
        operation: &'static str,
        subject: impl Into<String>,
        source: io::Error,
    ) -> Self {
        Self::Io {
            operation,
            subject: subject.into(),
            source,
        }
    }

    pub(crate) fn compiler_error(&self) -> Option<&compiler::CompileError> {
        match self {
            Self::Compiler(source) => Some(source),
            _ => None,
        }
    }

    pub(crate) fn rustc_diagnostics(&self) -> Option<&str> {
        match self {
            Self::NativeRuntime(source) => source.rustc_diagnostics(),
            _ => None,
        }
    }
}

impl fmt::Display for BackendSupportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                operation,
                subject,
                source,
            } => {
                write!(f, "failed to {operation} {subject}: {source}")
            }
            Self::Compiler(source) => write!(f, "application compilation failed: {source}"),
            Self::NativeRuntime(source) => {
                write!(f, "native runtime initialization failed: {source}")
            }
            Self::ResourceProfile(source) => {
                write!(f, "resource profile configuration failed: {source}")
            }
            Self::StaticPrefix(source) => write!(f, "invalid static URL prefix: {source}"),
            Self::Storage(source) => write!(f, "application filesystem setup failed: {source}"),
            Self::UnsafeUnixSocketPath(path) => write!(
                f,
                "refusing to replace non-socket Unix listener path `{}`",
                path.display()
            ),
            #[cfg(not(unix))]
            Self::UnixSocketsUnsupported => {
                write!(f, "Unix sockets are not supported on this platform")
            }
            Self::StaticRouteConflict { route, prefix } => write!(
                f,
                "application route `{route}` conflicts with static URL prefix `{prefix}`"
            ),
            Self::ReservedMediaRoute => write!(
                f,
                "application route conflicts with reserved media endpoint `/__velran/media/`"
            ),
            Self::ReservedHealthRoute { path } => write!(
                f,
                "application route conflicts with reserved health endpoint `{path}`"
            ),
            Self::HealthStaticConflict { path, prefix } => write!(
                f,
                "health endpoint `{path}` conflicts with static URL prefix `{prefix}`"
            ),
            Self::MissingUploadDataRoot => write!(
                f,
                "application declares an Upload route, but no data root is configured"
            ),
            Self::UploadPermissions => write!(
                f,
                "Upload routes require AppFs `c` and `w` permissions for atomic create and rollback cleanup"
            ),
            Self::ImageUploadPermissions => write!(
                f,
                "Image upload routes require AppFs `r`, `w`, and `c` permissions for magic-byte validation and serving"
            ),
        }
    }
}

impl Error for BackendSupportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Compiler(source) => Some(source),
            Self::NativeRuntime(source) => Some(source),
            Self::ResourceProfile(source) => Some(source),
            Self::StaticPrefix(source) => Some(source),
            Self::Storage(source) => Some(source),
            _ => None,
        }
    }
}

impl From<compiler::CompileError> for BackendSupportError {
    fn from(value: compiler::CompileError) -> Self {
        Self::Compiler(value)
    }
}
impl From<super::ResourceProfileConfigError> for BackendSupportError {
    fn from(value: super::ResourceProfileConfigError) -> Self {
        Self::ResourceProfile(value)
    }
}
impl From<super::StaticPrefixError> for BackendSupportError {
    fn from(value: super::StaticPrefixError) -> Self {
        Self::StaticPrefix(value)
    }
}
impl From<storage::FsError> for BackendSupportError {
    fn from(value: storage::FsError) -> Self {
        Self::Storage(value)
    }
}

impl From<native_runtime::application_runtime::RuntimeError> for BackendSupportError {
    fn from(value: native_runtime::application_runtime::RuntimeError) -> Self {
        Self::NativeRuntime(value)
    }
}
