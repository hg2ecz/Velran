use crate::events::{SecurityEvent, SystemEvent, json_line, utc_timestamp};
use crate::metrics::increment_log_fallback;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{SyncSender, TrySendError, channel, sync_channel};
use std::sync::{Arc, OnceLock};

#[derive(Debug, Clone, Default)]
pub struct LogConfig {
    pub server_file: Option<PathBuf>,
    pub access_file: Option<PathBuf>,
    pub audit_file: Option<PathBuf>,
    pub stderr: bool,
}

struct LogFiles {
    server: Option<File>,
    access: Option<File>,
    audit: Option<File>,
}
#[derive(Clone, Copy)]
enum LogKind {
    Server,
    Access,
    Audit,
}
enum LogCommand {
    Line(LogKind, String),
    Reopen(std::sync::mpsc::Sender<io::Result<()>>),
    Flush(std::sync::mpsc::Sender<()>),
}

pub struct LogManager {
    config: LogConfig,
    tx: SyncSender<LogCommand>,
}

fn open_log_file(path: &Path) -> io::Result<File> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "log path has no parent"))?;
    if !parent.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "log parent directory does not exist",
        ));
    }
    let mut opts = OpenOptions::new();
    opts.create(true).append(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o640)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    let file = opts.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "log target is not a regular file",
        ));
    }
    Ok(file)
}
fn open_log_files(config: &LogConfig) -> io::Result<LogFiles> {
    Ok(LogFiles {
        server: config
            .server_file
            .as_deref()
            .map(open_log_file)
            .transpose()?,
        access: config
            .access_file
            .as_deref()
            .map(open_log_file)
            .transpose()?,
        audit: config
            .audit_file
            .as_deref()
            .map(open_log_file)
            .transpose()?,
    })
}
fn write_line(files: &mut LogFiles, config: &LogConfig, kind: LogKind, line: &str) {
    let target = match kind {
        LogKind::Server => files.server.as_mut(),
        LogKind::Access => files.access.as_mut(),
        LogKind::Audit => files.audit.as_mut(),
    };
    let wrote = target
        .map(|f| writeln!(f, "{line}").is_ok())
        .unwrap_or(false);
    if !wrote {
        increment_log_fallback();
    }
    if config.stderr || !wrote {
        eprintln!("{line}");
    }
}
impl LogManager {
    pub fn new(config: LogConfig) -> io::Result<Self> {
        let mut files = open_log_files(&config)?;
        let (tx, rx) = sync_channel::<LogCommand>(8192);
        let thread_config = config.clone();
        std::thread::Builder::new()
            .name("velran-log-writer".into())
            .spawn(move || {
                while let Ok(cmd) = rx.recv() {
                    match cmd {
                        LogCommand::Line(kind, line) => {
                            write_line(&mut files, &thread_config, kind, &line)
                        }
                        LogCommand::Reopen(ack) => {
                            let result = open_log_files(&thread_config).map(|new_files| {
                                files = new_files;
                            });
                            let _ = ack.send(result);
                        }
                        LogCommand::Flush(ack) => {
                            for file in [&mut files.server, &mut files.access, &mut files.audit]
                                .into_iter()
                                .flatten()
                            {
                                let _ = file.flush();
                            }
                            let _ = ack.send(());
                        }
                    }
                }
            })?;
        Ok(Self { config, tx })
    }
    fn write_kind(&self, kind: LogKind, line: &str) {
        match self.tx.try_send(LogCommand::Line(kind, line.to_string())) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                increment_log_fallback();
                eprintln!("{line}")
            }
        }
    }
    pub fn server(&self, line: &str) {
        self.write_kind(LogKind::Server, line)
    }
    pub fn access(&self, line: &str) {
        self.write_kind(LogKind::Access, line)
    }
    pub fn audit(&self, line: &str) {
        self.write_kind(LogKind::Audit, line)
    }
    pub fn reopen(&self) -> io::Result<()> {
        let (ack_tx, ack_rx) = channel();
        self.tx
            .send(LogCommand::Reopen(ack_tx))
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "log writer stopped"))?;
        ack_rx
            .recv()
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "log writer stopped"))?
    }
    pub fn flush(&self) -> io::Result<()> {
        let (ack_tx, ack_rx) = channel();
        self.tx
            .send(LogCommand::Flush(ack_tx))
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "log writer stopped"))?;
        ack_rx
            .recv()
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "log writer stopped"))?;
        Ok(())
    }
    pub fn config(&self) -> &LogConfig {
        &self.config
    }
}
static GLOBAL_LOGGER: OnceLock<Arc<LogManager>> = OnceLock::new();
pub fn init_logging(config: LogConfig) -> io::Result<Arc<LogManager>> {
    let logger = Arc::new(LogManager::new(config)?);
    GLOBAL_LOGGER
        .set(Arc::clone(&logger))
        .map_err(|_| io::Error::new(io::ErrorKind::AlreadyExists, "logging already initialized"))?;
    Ok(logger)
}
pub fn server_event(level: &str, event: &str, component: &str, message: &str) {
    let line = json_line(&SystemEvent {
        schema_version: 1,
        timestamp: utc_timestamp(),
        event,
        level,
        component,
        message,
    })
    .unwrap_or_else(|_| format!(r#"{{"event":"log_serialization_failed","level":"error"}}"#));
    if let Some(l) = GLOBAL_LOGGER.get() {
        l.server(&line)
    } else {
        eprintln!("{line}")
    }
}
pub fn security_event(
    level: &str,
    category: &str,
    action: &str,
    outcome: &str,
    request_id: &str,
    route: &str,
) {
    let line = json_line(&SecurityEvent {
        schema_version: 1,
        timestamp: utc_timestamp(),
        event: "security_event",
        level,
        category,
        action,
        outcome,
        request_id,
        route,
        subject: "[redacted]",
        source: "[redacted]",
    })
    .unwrap_or_else(|_| format!(r#"{{"event":"log_serialization_failed","level":"error"}}"#));
    if let Some(l) = GLOBAL_LOGGER.get() {
        l.audit(&line)
    } else {
        eprintln!("{line}")
    }
}

pub fn server_log(line: &str) {
    server_event("info", "server_message", "server", line)
}
pub fn access_log(line: &str) {
    if let Some(l) = GLOBAL_LOGGER.get() {
        l.access(line)
    } else {
        eprintln!("{line}")
    }
}
pub fn audit_log(line: &str) {
    if let Some(l) = GLOBAL_LOGGER.get() {
        l.audit(line)
    } else {
        eprintln!("{line}")
    }
}
pub fn reopen_logs() -> io::Result<()> {
    match GLOBAL_LOGGER.get() {
        Some(l) => l.reopen(),
        None => Ok(()),
    }
}
pub fn flush_logs() -> io::Result<()> {
    match GLOBAL_LOGGER.get() {
        Some(l) => l.flush(),
        None => Ok(()),
    }
}

#[cfg(test)]
mod log_reopen_tests {
    use super::{LogConfig, LogManager};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn reopen_recreates_rotated_file() {
        let base = std::env::temp_dir().join(format!(
            "velran-log-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::create_dir_all(&base);
        let path = base.join("access.log");
        let rotated = base.join("access.log.1");
        let manager = LogManager::new(LogConfig {
            access_file: Some(path.clone()),
            stderr: false,
            ..Default::default()
        })
        .unwrap();
        manager.access("first");
        manager.flush().unwrap();
        std::fs::rename(&path, &rotated).unwrap();
        manager.reopen().unwrap();
        manager.access("second");
        manager.flush().unwrap();
        assert!(std::fs::read_to_string(&rotated).unwrap().contains("first"));
        assert!(std::fs::read_to_string(&path).unwrap().contains("second"));
        let _ = std::fs::remove_dir_all(base);
    }
}
