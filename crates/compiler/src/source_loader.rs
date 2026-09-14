use crate::diagnostics::CompileError;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

mod reexports;
mod tree;

use reexports::apply_public_reexports;
use tree::load_tree;
#[derive(Debug, Clone)]
pub(crate) struct SourceUnit {
    pub(crate) path: PathBuf,
    pub(crate) module_path: Vec<String>,
    pub(crate) namespace: String,
    pub(crate) source: String,
    pub(crate) module_declarations: Vec<crate::module_header::ModuleDecl>,
    pub(crate) public_reexports: Vec<crate::module_header::UseDecl>,
    pub(crate) package_root: Vec<String>,
    pub(crate) raw_sha256: String,
    pub(crate) raw_len: u64,
    pub(crate) raw_modified: Option<SystemTime>,
}
impl SourceUnit {
    pub(crate) fn namespace(&self) -> &str {
        &self.namespace
    }
}
#[derive(Debug)]
pub(crate) struct LoadedApplication {
    pub(crate) units: Vec<SourceUnit>,
    pub(crate) manifest_files: Vec<PathBuf>,
    pub(crate) source_roots: Vec<PathBuf>,
}

pub(crate) fn source_error(unit: &SourceUnit, err: CompileError) -> CompileError {
    if unit.path == Path::new("<memory>") {
        return err;
    }
    CompileError::located(unit.path.clone(), err)
}

pub(crate) fn load_application(entry: &Path) -> Result<LoadedApplication, CompileError> {
    let meta = fs::symlink_metadata(entry)?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(CompileError::Syntax(
            "application entrypoint must be a regular non-symlink .vrn file".into(),
        ));
    }
    if entry.extension().and_then(|v| v.to_str()) != Some("vrn") {
        return Err(CompileError::Syntax(
            "application entrypoint must use the .vrn extension".into(),
        ));
    }
    let entry = entry.canonicalize()?;
    let app_root = entry
        .parent()
        .ok_or_else(|| {
            CompileError::Syntax("application entrypoint has no parent directory".into())
        })?
        .to_path_buf();
    let package = crate::package_manifest::load_application(&entry)?;
    let mut units = Vec::new();
    let mut total = 0usize;
    load_tree(&entry, &app_root, &[], &mut units, &mut total)?;
    let mut roots = vec![app_root.clone()];
    for dep in &package.dependencies {
        if units.iter().any(|u| u.module_path == dep.namespace) {
            return Err(CompileError::Syntax(format!(
                "dependency `{}` conflicts with module `{}`",
                dep.name,
                dep.namespace.join("::")
            )));
        }
        load_tree(
            &dep.entry,
            &dep.root,
            &dep.namespace,
            &mut units,
            &mut total,
        )?;
        roots.push(dep.root.clone());
    }
    for dep in &package.dependencies {
        let owner = units
            .iter_mut()
            .find(|u| u.module_path == dep.parent_namespace)
            .ok_or_else(|| {
                CompileError::Syntax(format!(
                    "dependency `{}` has no declaring package root",
                    dep.name
                ))
            })?;
        owner
            .module_declarations
            .push(crate::module_header::ModuleDecl {
                path: dep.namespace.clone(),
                visibility: if dep.parent_namespace.is_empty() {
                    language_core::Visibility::Public
                } else {
                    language_core::Visibility::Private
                },
            });
    }
    apply_public_reexports(&mut units)?;
    roots.sort();
    roots.dedup();
    units.sort_by(|a, b| {
        a.module_path
            .cmp(&b.module_path)
            .then_with(|| a.path.cmp(&b.path))
    });
    Ok(LoadedApplication {
        units,
        manifest_files: package.manifests,
        source_roots: roots,
    })
}
