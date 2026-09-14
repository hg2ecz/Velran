use super::SourceUnit;
use crate::diagnostics::CompileError;
use crate::source_cache;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_MODULES: usize = 512;
const MAX_MODULE_BYTES: usize = 1024 * 1024;
const MAX_APP_SOURCE_BYTES: usize = 8 * 1024 * 1024;

pub(super) fn load_tree(
    entry: &Path,
    root: &Path,
    prefix: &[String],
    units: &mut Vec<SourceUnit>,
    total: &mut usize,
) -> Result<(), CompileError> {
    let mut seen = HashSet::new();
    let mut active = Vec::new();
    load_source_unit(
        entry,
        root,
        prefix.to_vec(),
        prefix,
        units,
        &mut seen,
        &mut active,
        total,
    )?;
    for (path, module_path) in
        crate::source_discovery::discover_sources(root, entry, prefix, MAX_MODULES)?
    {
        load_source_unit(
            &path,
            root,
            module_path,
            prefix,
            units,
            &mut seen,
            &mut active,
            total,
        )?;
    }
    Ok(())
}

fn resolve_module(
    app_root: &Path,
    module_path: &[String],
    package_prefix: &[String],
) -> Result<PathBuf, CompileError> {
    let display = module_path.join("::");
    let relative = module_path.strip_prefix(package_prefix).ok_or_else(|| {
        CompileError::Syntax(format!("module `{display}` escapes package namespace"))
    })?;
    let mut candidate = app_root.to_path_buf();
    for segment in relative {
        candidate.push(segment);
    }
    candidate.set_extension("vrn");
    if !candidate.exists() {
        return Err(CompileError::Syntax(format!(
            "module `{display}` was not found at `{}`; module paths resolve from the declaring module; use `crate::` for an application-root path",
            candidate.display()
        )));
    }
    let meta = fs::symlink_metadata(&candidate)?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(CompileError::Syntax(format!(
            "module `{display}` must be a regular non-symlink file"
        )));
    }
    let canonical = candidate.canonicalize()?;
    if !canonical.starts_with(app_root) {
        return Err(CompileError::Syntax(format!(
            "module `{display}` escapes the application root"
        )));
    }
    Ok(canonical)
}

#[allow(clippy::too_many_arguments)]
fn load_source_unit(
    path: &Path,
    app_root: &Path,
    module_path: Vec<String>,
    package_prefix: &[String],
    units: &mut Vec<SourceUnit>,
    seen: &mut HashSet<PathBuf>,
    active: &mut Vec<PathBuf>,
    total: &mut usize,
) -> Result<(), CompileError> {
    if units.len() >= MAX_MODULES {
        return Err(CompileError::Syntax(format!(
            "application exceeds module limit ({MAX_MODULES})"
        )));
    }
    if active.iter().any(|v| v == path) {
        let mut chain: Vec<String> = active.iter().map(|v| v.display().to_string()).collect();
        chain.push(path.display().to_string());
        return Err(CompileError::Syntax(format!(
            "module cycle: {}",
            chain.join(" -> ")
        )));
    }
    if !seen.insert(path.to_path_buf()) {
        return Ok(());
    }
    let snapshot = crate::source_snapshot::read(path)?;
    let raw_sha256 = snapshot.sha256;
    let raw_len = snapshot.len;
    let raw_modified = snapshot.modified;
    let bytes = snapshot.bytes;
    if bytes.len() > MAX_MODULE_BYTES {
        return Err(CompileError::Syntax(format!(
            "module `{}` exceeds {} bytes",
            path.display(),
            MAX_MODULE_BYTES
        )));
    }
    *total = total
        .checked_add(bytes.len())
        .ok_or_else(|| CompileError::Syntax("application source size overflow".into()))?;
    if *total > MAX_APP_SOURCE_BYTES {
        return Err(CompileError::Syntax(format!(
            "application source exceeds {} bytes",
            MAX_APP_SOURCE_BYTES
        )));
    }
    let source_raw = String::from_utf8(bytes)
        .map_err(|_| CompileError::Syntax(format!("module `{}` is not UTF-8", path.display())))?;
    let cached_mods = source_cache::load_mods(source_raw.as_bytes());
    let source = crate::rustlike_surface::normalize(&source_raw)
        .map_err(|err| CompileError::located(path.to_path_buf(), err))?;
    crate::rust_surface_security::validate(&source)
        .map_err(|err| CompileError::located(path.to_path_buf(), err))?;
    if !package_prefix.is_empty() {
        crate::library_surface::validate(&source)
            .map_err(|err| CompileError::located(path.to_path_buf(), err))?;
    }
    let header = crate::module_header::parse_in_package(&source, &module_path, package_prefix)
        .map_err(|err| CompileError::located(path.to_path_buf(), err))?;
    let source = crate::module_header::blank_header_lines(&source, &header.header_lines);
    let source = crate::module_header::expand_import_aliases(&source, &header.uses);
    let parsed_mods: Vec<Vec<String>> = header
        .modules
        .iter()
        .map(|module| module.path.clone())
        .collect();
    let mods = match cached_mods {
        Some(cached) if cached == parsed_mods => cached,
        _ => {
            source_cache::store_mods(source_raw.as_bytes(), &parsed_mods);
            parsed_mods
        }
    };
    units.push(SourceUnit {
        path: path.to_path_buf(),
        module_path: module_path.clone(),
        namespace: module_path.join("::"),
        source,
        module_declarations: header.modules.clone(),
        public_reexports: header
            .uses
            .iter()
            .filter(|u| u.visibility == language_core::Visibility::Public)
            .cloned()
            .collect(),
        package_root: package_prefix.to_vec(),
        raw_sha256,
        raw_len,
        raw_modified,
    });
    active.push(path.to_path_buf());
    for child_path in mods {
        let child = resolve_module(app_root, &child_path, package_prefix)?;
        load_source_unit(
            &child,
            app_root,
            child_path,
            package_prefix,
            units,
            seen,
            active,
            total,
        )?;
    }
    active.pop();
    Ok(())
}
