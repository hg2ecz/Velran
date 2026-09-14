use crate::diagnostics::CompileError;
use crate::source_syntax::is_identifier;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

const MANIFEST: &str = "velran.toml";
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_DIRECT_DEPENDENCIES: usize = 64;
const MAX_PACKAGE_COUNT: usize = 128;
const MAX_PACKAGE_DEPTH: usize = 32;

type DependencyPaths = BTreeMap<String, String>;

#[derive(Debug, Clone)]
pub(crate) struct LocalDependency {
    pub(crate) name: String,
    pub(crate) root: PathBuf,
    pub(crate) entry: PathBuf,
    pub(crate) namespace: Vec<String>,
    pub(crate) parent_namespace: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ApplicationPackage {
    pub(crate) dependencies: Vec<LocalDependency>,
    pub(crate) manifests: Vec<PathBuf>,
}

#[derive(Debug)]
struct ParsedManifest {
    name: String,
    kind: String,
    entry: String,
    dependencies: DependencyPaths,
}

struct GraphLoader {
    workspace: PathBuf,
    app_root: PathBuf,
    dependencies: Vec<LocalDependency>,
    manifests: Vec<PathBuf>,
    active_roots: Vec<PathBuf>,
    package_names: HashMap<String, PathBuf>,
    package_roots: HashMap<PathBuf, String>,
}

pub(crate) fn load_application(entry: &Path) -> Result<ApplicationPackage, CompileError> {
    let app_root = entry.parent().ok_or_else(|| {
        CompileError::Syntax("application entrypoint has no parent directory".into())
    })?;
    let manifest_path = app_root.join(MANIFEST);
    if !manifest_path.exists() {
        return Ok(ApplicationPackage::default());
    }

    let app = parse_manifest(&manifest_path)?;
    if app.kind != "app" {
        return Err(CompileError::Syntax(
            "application manifest must declare package.kind = `app`".into(),
        ));
    }
    if resolve_entry(app_root, &app.entry)? != entry.canonicalize()? {
        return Err(CompileError::Syntax(
            "application manifest entry does not match requested entrypoint".into(),
        ));
    }
    enforce_dependency_limit(&app.name, app.dependencies.len())?;

    let app_root = app_root.canonicalize()?;
    let workspace = app_root.parent().unwrap_or(&app_root).canonicalize()?;
    let mut loader = GraphLoader {
        workspace,
        app_root: app_root.clone(),
        dependencies: Vec::new(),
        manifests: vec![manifest_path.canonicalize()?],
        active_roots: vec![app_root.clone()],
        package_names: HashMap::new(),
        package_roots: HashMap::new(),
    };
    loader
        .package_names
        .insert(app.name.clone(), app_root.clone());
    loader.package_roots.insert(app_root.clone(), app.name);
    loader.load_dependencies(&app_root, &[], app.dependencies, 1)?;

    loader.manifests.sort();
    loader.manifests.dedup();
    loader.dependencies.sort_by(|a, b| {
        a.namespace
            .cmp(&b.namespace)
            .then_with(|| a.root.cmp(&b.root))
    });
    Ok(ApplicationPackage {
        dependencies: loader.dependencies,
        manifests: loader.manifests,
    })
}

impl GraphLoader {
    fn load_dependencies(
        &mut self,
        declaring_root: &Path,
        parent_namespace: &[String],
        dependencies: DependencyPaths,
        depth: usize,
    ) -> Result<(), CompileError> {
        if depth > MAX_PACKAGE_DEPTH {
            return Err(CompileError::Syntax(format!(
                "local package graph exceeds maximum depth ({MAX_PACKAGE_DEPTH})"
            )));
        }
        enforce_dependency_limit(
            &display_namespace(parent_namespace, "application"),
            dependencies.len(),
        )?;

        for (name, raw) in dependencies {
            if !is_identifier(&name) {
                return Err(CompileError::Syntax(format!(
                    "invalid dependency name `{name}`"
                )));
            }
            if self.dependencies.len() >= MAX_PACKAGE_COUNT {
                return Err(CompileError::Syntax(format!(
                    "local package graph exceeds package limit ({MAX_PACKAGE_COUNT})"
                )));
            }

            let root = self.resolve_dependency_root(declaring_root, &raw)?;
            if let Some(position) = self.active_roots.iter().position(|active| active == &root) {
                let mut chain: Vec<String> = self.active_roots[position..]
                    .iter()
                    .map(|path| {
                        self.package_roots
                            .get(path)
                            .cloned()
                            .unwrap_or_else(|| path.display().to_string())
                    })
                    .collect();
                chain.push(name.clone());
                return Err(CompileError::Syntax(format!(
                    "local package dependency cycle: {}",
                    chain.join(" -> ")
                )));
            }

            let manifest_path = root.join(MANIFEST);
            let lib = parse_manifest(&manifest_path)?;
            if lib.kind != "lib" {
                return Err(CompileError::Syntax(format!(
                    "dependency `{name}` must declare package.kind = `lib`"
                )));
            }
            if lib.name != name {
                return Err(CompileError::Syntax(format!(
                    "dependency name `{name}` must match library package name `{}`",
                    lib.name
                )));
            }
            enforce_dependency_limit(&lib.name, lib.dependencies.len())?;
            self.register_identity(&name, &root)?;

            let mut namespace = parent_namespace.to_vec();
            namespace.push(name.clone());
            let entry = resolve_entry(&root, &lib.entry)?;
            self.manifests.push(manifest_path.canonicalize()?);
            self.dependencies.push(LocalDependency {
                name: name.clone(),
                root: root.clone(),
                entry,
                namespace: namespace.clone(),
                parent_namespace: parent_namespace.to_vec(),
            });

            self.active_roots.push(root.clone());
            self.load_dependencies(&root, &namespace, lib.dependencies, depth + 1)?;
            self.active_roots.pop();
        }
        Ok(())
    }

    fn resolve_dependency_root(
        &self,
        declaring_root: &Path,
        raw: &str,
    ) -> Result<PathBuf, CompileError> {
        let candidate = declaring_root.join(raw);
        let meta = fs::symlink_metadata(&candidate).map_err(|_| {
            CompileError::Syntax(format!("local dependency path `{raw}` does not exist"))
        })?;
        if meta.file_type().is_symlink() || !meta.is_dir() {
            return Err(CompileError::Syntax(format!(
                "local dependency path `{raw}` must be a regular non-symlink directory"
            )));
        }
        let root = candidate.canonicalize()?;
        if !root.starts_with(&self.workspace) || root.starts_with(&self.app_root) {
            return Err(CompileError::Syntax(format!(
                "local dependency path `{raw}` must stay inside the workspace and outside the application package"
            )));
        }
        Ok(root)
    }

    fn register_identity(&mut self, name: &str, root: &Path) -> Result<(), CompileError> {
        if let Some(existing) = self.package_names.get(name) {
            if existing != root {
                return Err(CompileError::Syntax(format!(
                    "local package name `{name}` resolves to multiple package roots"
                )));
            }
        }
        if let Some(existing) = self.package_roots.get(root) {
            if existing != name {
                return Err(CompileError::Syntax(format!(
                    "local package root `{}` is declared with both `{existing}` and `{name}`",
                    root.display()
                )));
            }
        }
        self.package_names
            .insert(name.to_string(), root.to_path_buf());
        self.package_roots
            .insert(root.to_path_buf(), name.to_string());
        Ok(())
    }
}

fn display_namespace(namespace: &[String], fallback: &str) -> String {
    if namespace.is_empty() {
        fallback.to_string()
    } else {
        namespace.join("::")
    }
}

fn enforce_dependency_limit(package: &str, count: usize) -> Result<(), CompileError> {
    if count > MAX_DIRECT_DEPENDENCIES {
        return Err(CompileError::Syntax(format!(
            "package `{package}` exceeds local dependency limit ({MAX_DIRECT_DEPENDENCIES})"
        )));
    }
    Ok(())
}

fn parse_manifest(path: &Path) -> Result<ParsedManifest, CompileError> {
    let meta = fs::symlink_metadata(path).map_err(|_| {
        CompileError::Syntax(format!("package manifest `{}` is required", path.display()))
    })?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(CompileError::Syntax(
            "package manifest must be a regular non-symlink file".into(),
        ));
    }
    if meta.len() > MAX_MANIFEST_BYTES {
        return Err(CompileError::Syntax("package manifest is too large".into()));
    }
    let text = fs::read_to_string(path)?;
    let value: toml::Value = toml::from_str(&text)
        .map_err(|e| CompileError::Syntax(format!("invalid package manifest: {e}")))?;
    let pkg = value
        .get("package")
        .and_then(|v| v.as_table())
        .ok_or_else(|| CompileError::Syntax("package manifest requires [package]".into()))?;
    let name = req(pkg, "name")?;
    if !is_identifier(&name) {
        return Err(CompileError::Syntax(format!(
            "invalid package name `{name}`"
        )));
    }
    let kind = req(pkg, "kind")?;
    if kind != "app" && kind != "lib" {
        return Err(CompileError::Syntax(
            "package.kind must be `app` or `lib`".into(),
        ));
    }
    let entry = pkg
        .get("entry")
        .and_then(|v| v.as_str())
        .unwrap_or(if kind == "app" { "main.vrn" } else { "lib.vrn" })
        .to_string();
    validate_entry(&entry)?;

    let mut dependencies = DependencyPaths::new();
    if let Some(value) = value.get("dependencies") {
        let table = value
            .as_table()
            .ok_or_else(|| CompileError::Syntax("[dependencies] must be a table".into()))?;
        for (name, spec) in table {
            if !is_identifier(name) {
                return Err(CompileError::Syntax(format!(
                    "invalid dependency name `{name}`"
                )));
            }
            let dependency = spec.as_table().ok_or_else(|| {
                CompileError::Syntax(format!("dependency `{name}` must use {{ path = \"...\" }}"))
            })?;
            if dependency.len() != 1 || !dependency.contains_key("path") {
                return Err(CompileError::Syntax(format!(
                    "dependency `{name}` may contain only a local path; git, registry, version, and build-script sources are not supported"
                )));
            }
            let raw = dependency["path"]
                .as_str()
                .ok_or_else(|| CompileError::Syntax("dependency path must be a string".into()))?;
            if Path::new(raw).is_absolute() {
                return Err(CompileError::Syntax(
                    "dependency path must be relative".into(),
                ));
            }
            dependencies.insert(name.clone(), raw.to_string());
        }
    }

    Ok(ParsedManifest {
        name,
        kind,
        entry,
        dependencies,
    })
}

fn req(table: &toml::map::Map<String, toml::Value>, key: &str) -> Result<String, CompileError> {
    table
        .get(key)
        .and_then(|value| value.as_str())
        .map(str::to_owned)
        .ok_or_else(|| CompileError::Syntax(format!("package.{key} must be a string")))
}

fn validate_entry(entry: &str) -> Result<(), CompileError> {
    let path = Path::new(entry);
    if path.is_absolute()
        || entry.contains("..")
        || entry.contains('\\')
        || path.extension().and_then(|value| value.to_str()) != Some("vrn")
    {
        return Err(CompileError::Syntax(format!(
            "invalid package entry `{entry}`"
        )));
    }
    Ok(())
}

fn resolve_entry(root: &Path, entry: &str) -> Result<PathBuf, CompileError> {
    let path = root.join(entry);
    let meta = fs::symlink_metadata(&path).map_err(|_| {
        CompileError::Syntax(format!("package entry `{}` does not exist", path.display()))
    })?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(CompileError::Syntax(
            "package entry must be a regular non-symlink file".into(),
        ));
    }
    let canonical = path.canonicalize()?;
    if !canonical.starts_with(root.canonicalize()?) {
        return Err(CompileError::Syntax(
            "package entry escapes package root".into(),
        ));
    }
    Ok(canonical)
}
