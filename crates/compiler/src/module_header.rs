use crate::diagnostics::CompileError;
use crate::source_syntax::is_identifier;
use language_core::Visibility;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModuleDecl {
    pub(crate) path: Vec<String>,
    pub(crate) visibility: Visibility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UseDecl {
    pub(crate) target: Vec<String>,
    pub(crate) alias: String,
    pub(crate) visibility: Visibility,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ModuleHeader {
    pub(crate) modules: Vec<ModuleDecl>,
    pub(crate) uses: Vec<UseDecl>,
    pub(crate) header_lines: Vec<usize>,
}

pub(crate) fn parse(source: &str, namespace: &[String]) -> Result<ModuleHeader, CompileError> {
    parse_in_package(source, namespace, &[])
}

pub(crate) fn parse_in_package(
    source: &str,
    namespace: &[String],
    package_root: &[String],
) -> Result<ModuleHeader, CompileError> {
    let mut header = ModuleHeader::default();
    let mut first_declaration_line = None;
    for (idx, raw) in source.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        if crate::declarations::starts_top_level_declaration(line) {
            first_declaration_line = Some(idx);
            break;
        }
        if let Some(rest) = line
            .strip_prefix("pub mod ")
            .or_else(|| line.strip_prefix("mod "))
        {
            let public = line.starts_with("pub mod ");
            let raw_path = rest
                .strip_suffix(';')
                .ok_or_else(|| {
                    CompileError::Syntax(format!(
                        "line {}: module declaration must end with `;`",
                        idx + 1
                    ))
                })?
                .trim();
            let path = resolve_module_path(raw_path, namespace, package_root)?;
            if header.modules.iter().any(|m| m.path == path) {
                return Err(CompileError::Syntax(format!(
                    "line {}: duplicate module declaration `{raw_path}`",
                    idx + 1
                )));
            }
            header.modules.push(ModuleDecl {
                path,
                visibility: if public {
                    Visibility::Public
                } else {
                    Visibility::Private
                },
            });
            header.header_lines.push(idx);
            continue;
        }
        let (use_visibility, use_rest) = if let Some(rest) = line.strip_prefix("pub use ") {
            (Visibility::Public, Some(rest))
        } else if let Some(rest) = line.strip_prefix("use ") {
            (Visibility::Private, Some(rest))
        } else {
            (Visibility::Private, None)
        };
        if let Some(rest) = use_rest {
            let rest = rest
                .strip_suffix(';')
                .ok_or_else(|| {
                    CompileError::Syntax(format!(
                        "line {}: use declaration must end with `;`",
                        idx + 1
                    ))
                })?
                .trim();
            let (raw_path, alias) = rest.split_once(" as ").ok_or_else(|| {
                CompileError::Syntax(format!(
                    "line {}: Velran imports require an explicit alias: `use path as alias;`",
                    idx + 1
                ))
            })?;
            if raw_path.contains('*') {
                return Err(CompileError::Syntax(format!(
                    "line {}: wildcard imports/re-exports are not supported",
                    idx + 1
                )));
            }
            if !is_identifier(alias.trim()) {
                return Err(CompileError::Syntax(format!(
                    "line {}: invalid import alias `{}`",
                    idx + 1,
                    alias.trim()
                )));
            }
            if use_visibility == Visibility::Public && namespace != package_root {
                return Err(CompileError::Syntax(format!(
                    "line {}: `pub use` is restricted to the package root in this iteration",
                    idx + 1
                )));
            }
            let target = resolve_use_path(raw_path.trim(), namespace, package_root)?;
            if header.uses.iter().any(|u| u.alias == alias.trim()) {
                return Err(CompileError::Syntax(format!(
                    "line {}: duplicate import alias `{}`",
                    idx + 1,
                    alias.trim()
                )));
            }
            header.uses.push(UseDecl {
                target,
                alias: alias.trim().to_string(),
                visibility: use_visibility,
            });
            header.header_lines.push(idx);
            continue;
        }
        return Err(CompileError::Syntax(format!(
            "line {}: only comments, `mod`, `pub mod`, or `use path as alias;` may appear before the first declaration",
            idx + 1
        )));
    }

    if let Some(first_declaration_line) = first_declaration_line {
        let mut offset = 0usize;
        for (idx, raw) in source.lines().enumerate() {
            let line_start = offset;
            offset += raw.len() + 1;
            if idx <= first_declaration_line {
                continue;
            }
            let line = raw.trim();
            let keyword_offset = raw.len() - raw.trim_start().len();
            let top_level = crate::declarations::is_top_level_declaration_at(
                source,
                (line_start + keyword_offset).min(source.len()),
            );
            if top_level
                && (line.starts_with("mod ")
                    || line.starts_with("pub mod ")
                    || line.starts_with("use ")
                    || line.starts_with("pub use "))
            {
                return Err(CompileError::Syntax(format!(
                    "line {}: module/import declarations must appear before the first code declaration",
                    idx + 1
                )));
            }
        }
    }
    Ok(header)
}

fn resolve_module_path(
    raw: &str,
    namespace: &[String],
    package_root: &[String],
) -> Result<Vec<String>, CompileError> {
    resolve_path(raw, namespace, package_root, true)
}
fn resolve_use_path(
    raw: &str,
    namespace: &[String],
    package_root: &[String],
) -> Result<Vec<String>, CompileError> {
    resolve_path(raw, namespace, package_root, false)
}
fn resolve_path(
    raw: &str,
    namespace: &[String],
    package_root: &[String],
    relative_default: bool,
) -> Result<Vec<String>, CompileError> {
    if raw.contains('/') || raw.contains('\\') || raw.contains("..") {
        return Err(CompileError::Syntax(format!("invalid module path `{raw}`")));
    }
    let mut base = if let Some(rest) = raw.strip_prefix("crate::") {
        return validated_segments(rest, package_root.to_vec());
    } else if let Some(rest) = raw.strip_prefix("self::") {
        return validated_segments(rest, namespace.to_vec());
    } else if let Some(rest) = raw.strip_prefix("super::") {
        let mut parent = namespace.to_vec();
        if parent.len() <= package_root.len() {
            return Err(CompileError::Syntax(
                "`super::` cannot escape the package root".into(),
            ));
        }
        parent.pop();
        return validated_segments(rest, parent);
    } else if relative_default {
        namespace.to_vec()
    } else {
        package_root.to_vec()
    };
    validated_segments(raw, std::mem::take(&mut base))
}
fn validated_segments(raw: &str, mut base: Vec<String>) -> Result<Vec<String>, CompileError> {
    let parts: Vec<&str> = raw.split("::").collect();
    if parts.is_empty() || parts.iter().any(|s| !is_identifier(s.trim())) {
        return Err(CompileError::Syntax(format!("invalid module path `{raw}`")));
    }
    base.extend(parts.into_iter().map(|s| s.trim().to_string()));
    Ok(base)
}

pub(crate) fn blank_header_lines(source: &str, lines: &[usize]) -> String {
    let set: std::collections::HashSet<usize> = lines.iter().copied().collect();
    source
        .split_inclusive('\n')
        .enumerate()
        .map(|(i, raw)| {
            if !set.contains(&i) {
                return raw.to_string();
            }
            let (line, nl) = raw.strip_suffix('\n').map_or((raw, ""), |s| (s, "\n"));
            format!("{}{}", " ".repeat(line.len()), nl)
        })
        .collect()
}

pub(crate) fn expand_import_aliases(source: &str, uses: &[UseDecl]) -> String {
    if uses.is_empty() {
        return source.to_string();
    }
    let mut out = source.to_string();
    for use_decl in uses {
        let from = format!("{}::", use_decl.alias);
        let to = format!("{}::", use_decl.target.join("::"));
        out = replace_token_prefix(&out, &from, &to);
    }
    out
}

pub(crate) fn replace_token_prefix(source: &str, from: &str, to: &str) -> String {
    let bytes = source.as_bytes();
    let from = from.as_bytes();
    let to = to.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    while i < bytes.len() {
        if in_string {
            let b = bytes[i];
            out.push(b);
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if bytes[i] == b'"' {
            in_string = true;
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'/') {
            while i < bytes.len() {
                let b = bytes[i];
                out.push(b);
                i += 1;
                if b == b'\n' {
                    break;
                }
            }
            continue;
        }
        if bytes[i..].starts_with(from) {
            let before = i.checked_sub(1).and_then(|index| bytes.get(index)).copied();
            if !before.is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_') {
                out.extend_from_slice(to);
                i += from.len();
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).expect("import expansion preserves UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_mod_is_relative_and_crate_path_is_absolute() {
        let ns = vec!["pages".to_string()];
        let header = parse("mod article;\nmod crate::shared;\n", &ns).unwrap();
        assert_eq!(header.modules[0].path, vec!["pages", "article"]);
        assert_eq!(header.modules[1].path, vec!["shared"]);
    }

    #[test]
    fn alias_expansion_ignores_strings_and_comments() {
        let uses = vec![UseDecl {
            target: vec!["catalog".into(), "math".into()],
            alias: "m".into(),
            visibility: Visibility::Private,
        }];
        let source = "let x = m::answer();\n// m::ignored\nlet s = \"m::ignored\";\n";
        let expanded = expand_import_aliases(source, &uses);
        assert!(expanded.contains("catalog::math::answer()"));
        assert!(expanded.contains("// m::ignored"));
        assert!(expanded.contains("\"m::ignored\""));
    }

    #[test]
    fn root_pub_use_is_parsed_as_explicit_reexport() {
        let header = parse("pub use catalog::math as math;\n", &[]).unwrap();
        assert_eq!(header.uses.len(), 1);
        assert_eq!(header.uses[0].visibility, Visibility::Public);
        assert_eq!(header.uses[0].target, vec!["catalog", "math"]);
    }

    #[test]
    fn nested_pub_use_is_fail_closed() {
        let err = parse_in_package("pub use crate::shared as shared;\n", &["pages".into()], &[])
            .unwrap_err();
        assert!(err.to_string().contains("package root"));
    }

    #[test]
    fn wildcard_pub_use_is_fail_closed() {
        let err = parse("pub use catalog::* as catalog;\n", &[]).unwrap_err();
        assert!(err.to_string().contains("wildcard"));
    }
}
