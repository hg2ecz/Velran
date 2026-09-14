use crate::diagnostics::CompileError;
use crate::source_syntax::is_identifier;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn discover_sources(
    app_root: &Path,
    entry: &Path,
    namespace_prefix: &[String],
    max_modules: usize,
) -> Result<Vec<(PathBuf, Vec<String>)>, CompileError> {
    let mut pending = vec![app_root.to_path_buf()];
    let mut discovered = Vec::new();
    while let Some(dir) = pending.pop() {
        let mut entries: Vec<_> = fs::read_dir(&dir)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for item in entries {
            let path = item.path();
            let meta = fs::symlink_metadata(&path)?;
            if meta.file_type().is_symlink() {
                if path.extension().and_then(|v| v.to_str()) == Some("vrn") || path.is_dir() {
                    return Err(CompileError::Syntax(format!(
                        "symlinks are not allowed in the Velran source tree: `{}`",
                        path.display()
                    )));
                }
                continue;
            }
            if meta.is_dir() {
                let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                    continue;
                };
                if is_identifier(name) {
                    pending.push(path);
                }
                continue;
            }
            if !meta.is_file() || path.extension().and_then(|v| v.to_str()) != Some("vrn") {
                continue;
            }
            let canonical = path.canonicalize()?;
            if canonical == entry {
                continue;
            }
            if !canonical.starts_with(app_root) {
                return Err(CompileError::Syntax(format!(
                    "discovered source escapes the application root: `{}`",
                    canonical.display()
                )));
            }
            let relative = canonical.strip_prefix(app_root).map_err(|_| {
                CompileError::Syntax("discovered source escapes the application root".into())
            })?;
            let mut module_path = namespace_prefix.to_vec();
            for component in relative.components() {
                let raw = component.as_os_str().to_str().ok_or_else(|| {
                    CompileError::Syntax(format!(
                        "source path is not UTF-8: `{}`",
                        canonical.display()
                    ))
                })?;
                let segment = raw.strip_suffix(".vrn").unwrap_or(raw);
                if !is_identifier(segment) {
                    return Err(CompileError::Syntax(format!(
                        "source path component `{segment}` is not a valid Velran module identifier"
                    )));
                }
                module_path.push(segment.to_owned());
            }
            discovered.push((canonical, module_path));
        }
    }
    discovered.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    if discovered.len().saturating_add(1) > max_modules {
        return Err(CompileError::Syntax(format!(
            "package exceeds module limit ({max_modules})"
        )));
    }
    Ok(discovered)
}
