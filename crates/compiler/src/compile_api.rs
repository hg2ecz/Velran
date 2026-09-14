use crate::diagnostics::CompileError;
use crate::{compile_units, source_loader};
use executable_ir::{VerifiedExecutableProgram, VerifyError};
use language_core::Program;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledSource {
    pub path: PathBuf,
    pub sha256: String,
    pub len: u64,
    pub modified: Option<SystemTime>,
}

#[derive(Debug)]
pub struct CompiledFile {
    pub program: Program,
    pub source_files: Vec<PathBuf>,
    pub source_roots: Vec<PathBuf>,
    pub sources: Vec<CompiledSource>,
}

pub fn compile_file(path: impl AsRef<Path>) -> Result<Program, CompileError> {
    Ok(compile_file_with_dependencies(path)?.program)
}

pub fn compile_file_with_dependencies(
    path: impl AsRef<Path>,
) -> Result<CompiledFile, CompileError> {
    let loaded = source_loader::load_application(path.as_ref())?;
    let mut source_files: Vec<PathBuf> =
        loaded.units.iter().map(|unit| unit.path.clone()).collect();
    source_files.extend(loaded.manifest_files.iter().cloned());
    source_files.sort();
    source_files.dedup();
    let mut sources: Vec<CompiledSource> = loaded
        .units
        .iter()
        .map(|unit| CompiledSource {
            path: unit.path.clone(),
            sha256: unit.raw_sha256.clone(),
            len: unit.raw_len,
            modified: unit.raw_modified,
        })
        .collect();
    for manifest in &loaded.manifest_files {
        let snap = crate::source_snapshot::read(manifest)?;
        sources.push(CompiledSource {
            path: manifest.clone(),
            sha256: snap.sha256,
            len: snap.len,
            modified: snap.modified,
        });
    }
    sources.sort_by(|a, b| a.path.cmp(&b.path));
    let program = compile_units(&loaded.units)?;
    Ok(CompiledFile {
        program,
        source_files,
        source_roots: loaded.source_roots,
        sources,
    })
}

pub fn compile_source(source: &str) -> Result<Program, CompileError> {
    let source = crate::rustlike_surface::normalize(source)?;
    crate::rust_surface_security::validate(&source)?;
    let header = crate::module_header::parse(&source, &Vec::new())?;
    if !header.modules.is_empty() || !header.uses.is_empty() {
        return Err(CompileError::Syntax(
            "`mod`/`use` requires file compilation; use compile_file/main.vrn".into(),
        ));
    }
    let units = vec![source_loader::SourceUnit {
        path: PathBuf::from("<memory>"),
        module_path: Vec::new(),
        namespace: String::new(),
        raw_sha256: String::new(),
        raw_len: source.len() as u64,
        raw_modified: None,
        source,
        module_declarations: Vec::new(),
        public_reexports: Vec::new(),
        package_root: Vec::new(),
    }];
    compile_units(&units)
}

#[derive(Debug)]
pub enum VerifiedCompileError {
    Compile(CompileError),
    Verify(VerifyError),
}

pub fn compile_verified_file(
    path: impl AsRef<Path>,
) -> Result<VerifiedExecutableProgram, VerifiedCompileError> {
    let program = compile_file(path).map_err(VerifiedCompileError::Compile)?;
    executable_ir::verify(&program).map_err(VerifiedCompileError::Verify)
}

pub fn compile_verified_source(
    source: &str,
) -> Result<VerifiedExecutableProgram, VerifiedCompileError> {
    let program = compile_source(source).map_err(VerifiedCompileError::Compile)?;
    executable_ir::verify(&program).map_err(VerifiedCompileError::Verify)
}
