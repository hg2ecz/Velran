use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum CompileError {
    Io(std::io::Error),
    Syntax(String),
    DuplicateHandler(String),
    DuplicateRoute(String),
    UnknownHandler(String),
    RouteParamMismatch(String),
    UnknownVariable(String),
    UnsafeSql(String),
    UnsafeHtml(String),
    Security {
        code: &'static str,
        message: String,
        help: Option<String>,
    },
    UnknownQuery(String),
    UnknownModel(String),
    Located {
        path: PathBuf,
        line: Option<usize>,
        source: Box<CompileError>,
    },
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Syntax(v) => write!(f, "syntax error: {v}"),
            Self::DuplicateHandler(v) => write!(f, "duplicate handler `{v}`"),
            Self::DuplicateRoute(v) => write!(f, "duplicate route `{v}`"),
            Self::UnknownHandler(v) => write!(f, "route references unknown handler `{v}`"),
            Self::RouteParamMismatch(v) => write!(f, "route parameter mismatch: {v}"),
            Self::UnknownVariable(v) => write!(f, "unknown variable `{v}`"),
            Self::UnsafeSql(v) => write!(f, "security error[SEC-A05-001]: unsafe SQL: {v}"),
            Self::UnsafeHtml(v) => write!(f, "security error[SEC-A05-002]: unsafe HTML: {v}"),
            Self::Security {
                code,
                message,
                help,
            } => {
                write!(f, "security error[{code}]: {message}")?;
                if let Some(help) = help {
                    write!(f, "\nhelp: {help}")?;
                }
                Ok(())
            }
            Self::UnknownQuery(v) => write!(f, "unknown query `{v}`"),
            Self::UnknownModel(v) => write!(f, "unknown model `{v}`"),
            Self::Located { path, line, source } => {
                if let Some(line) = line {
                    write!(f, "{}:{}: {source}", path.display(), line)
                } else {
                    write!(f, "{}: {source}", path.display())
                }
            }
        }
    }
}

impl CompileError {
    pub(crate) fn security(
        code: &'static str,
        message: impl Into<String>,
        help: impl Into<Option<String>>,
    ) -> Self {
        Self::Security {
            code,
            message: message.into(),
            help: help.into(),
        }
    }

    pub(crate) fn located(path: PathBuf, err: CompileError) -> Self {
        let line = err.line_hint();
        Self::Located {
            path,
            line,
            source: Box::new(err),
        }
    }

    fn line_hint(&self) -> Option<usize> {
        let text = match self {
            Self::Syntax(v)
            | Self::RouteParamMismatch(v)
            | Self::UnsafeSql(v)
            | Self::UnsafeHtml(v) => v.as_str(),
            Self::Security { message, .. } => message.as_str(),
            Self::Located { line, .. } => return *line,
            _ => return None,
        };
        parse_line_hint(text)
    }

    pub fn render_debug(&self, app_entry: &Path) -> String {
        let root = app_entry.parent().unwrap_or_else(|| Path::new("."));
        let mut out = String::new();
        render_error(self, root, &mut out);
        out
    }
}

fn parse_line_hint(text: &str) -> Option<usize> {
    let rest = text.strip_prefix("line ")?;
    let digits: String = rest.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

fn render_error(err: &CompileError, root: &Path, out: &mut String) {
    if let CompileError::Located { path, line, source } = err {
        let fallback_path = path
            .file_name()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("<source>"));
        let display_path = match path.strip_prefix(root) {
            Ok(value) if !value.as_os_str().is_empty() => value,
            _ => fallback_path.as_path(),
        };
        let code = diagnostic_code(source);
        let message = terminal_safe(&diagnostic_message(source), 512);
        let safe_path = terminal_safe(&display_path.display().to_string(), 240);
        out.push_str(&format!("error[{code}]: {message}\n"));
        if let Some(line_no) = *line {
            out.push_str(&format!("  --> {safe_path}:{line_no}\n"));
            if let Ok(text) = fs::read_to_string(path) {
                if let Some(source_line) = text.lines().nth(line_no.saturating_sub(1)) {
                    out.push_str("   |\n");
                    let source_line = terminal_safe(source_line, 240);
                    out.push_str(&format!("{line_no:>3} | {source_line}\n"));
                    out.push_str("   | ^\n");
                }
            }
        } else {
            out.push_str(&format!("  --> {safe_path}\n"));
        }
        if let CompileError::Security {
            help: Some(help), ..
        } = source.as_ref()
        {
            out.push_str(&format!("   = help: {}\n", terminal_safe(help, 512)));
        }
        return;
    }
    out.push_str(&format!(
        "error[{}]: {}\n",
        diagnostic_code(err),
        terminal_safe(&diagnostic_message(err), 512)
    ));
}

fn terminal_safe(value: &str, max_chars: usize) -> String {
    let mut out = String::with_capacity(value.len().min(max_chars));
    for ch in value.chars().take(max_chars) {
        if ch == '\t' {
            out.push_str("    ");
        } else if ch.is_control() {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    if value.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

fn diagnostic_code(err: &CompileError) -> &'static str {
    match err {
        CompileError::Syntax(_) => "E0001",
        CompileError::DuplicateHandler(_) => "E0101",
        CompileError::DuplicateRoute(_) => "E0102",
        CompileError::UnknownHandler(_) => "E0103",
        CompileError::RouteParamMismatch(_) => "E0201",
        CompileError::UnknownVariable(_) => "E0202",
        CompileError::UnsafeSql(_) => "SEC-A05-001",
        CompileError::UnsafeHtml(_) => "SEC-A05-002",
        CompileError::Security { code, .. } => code,
        CompileError::UnknownQuery(_) => "E0301",
        CompileError::UnknownModel(_) => "E0302",
        CompileError::Io(_) => "E9001",
        CompileError::Located { source, .. } => diagnostic_code(source),
    }
}

fn diagnostic_message(err: &CompileError) -> String {
    match err {
        CompileError::Security { message, .. } => message.clone(),
        CompileError::Located { source, .. } => diagnostic_message(source),
        _ => err.to_string(),
    }
}

impl std::error::Error for CompileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(source) => Some(source),
            Self::Located { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

impl From<std::io::Error> for CompileError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
