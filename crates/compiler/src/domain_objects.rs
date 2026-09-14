use crate::declarations;
use crate::diagnostics::CompileError;
use crate::source_loader::SourceUnit;
use crate::source_loader::source_error;
use crate::source_syntax::{is_identifier, matching_brace, read_ident};
use std::collections::HashSet;

#[derive(Debug, Clone)]
struct DomainObjectSpec {
    name: String,
    open: usize,
    close: usize,
    members: Vec<String>,
}

pub(super) fn prepare_domain_units(units: &[SourceUnit]) -> Result<Vec<SourceUnit>, CompileError> {
    let mut specs_by_unit = Vec::with_capacity(units.len());
    let mut object_names = HashSet::new();
    for unit in units {
        let specs = scan_domain_objects(&unit.source).map_err(|e| source_error(unit, e))?;
        for spec in &specs {
            let qualified = if unit.module_path.is_empty() {
                spec.name.clone()
            } else {
                format!("{}::{}", unit.namespace(), spec.name)
            };
            if !object_names.insert(qualified) {
                return Err(source_error(
                    unit,
                    CompileError::Syntax(format!("duplicate domain object `{}`", spec.name)),
                ));
            }
        }
        specs_by_unit.push(specs);
    }
    units
        .iter()
        .zip(specs_by_unit.iter())
        .map(|(unit, specs)| {
            Ok(SourceUnit {
                path: unit.path.clone(),
                module_path: unit.module_path.clone(),
                namespace: unit.namespace().to_owned(),
                source: expand_domain_source(&unit.source, specs)
                    .map_err(|e| source_error(unit, e))?,
                module_declarations: unit.module_declarations.clone(),
                public_reexports: unit.public_reexports.clone(),
                package_root: unit.package_root.clone(),
                raw_sha256: unit.raw_sha256.clone(),
                raw_len: unit.raw_len,
                raw_modified: unit.raw_modified,
            })
        })
        .collect()
}

fn scan_domain_objects(source: &str) -> Result<Vec<DomainObjectSpec>, CompileError> {
    let mut out = Vec::new();
    let mut cursor = 0usize;
    while let Some(rel) = source[cursor..].find("object ") {
        let pos = cursor + rel;
        if !declarations::is_top_level_declaration_at(source, pos) {
            cursor = pos + 7;
            continue;
        }
        if pos > 0 {
            let prev = source.as_bytes()[pos - 1];
            if prev.is_ascii_alphanumeric() || prev == b'_' {
                cursor = pos + 7;
                continue;
            }
        }
        let line_start = source[..pos].rfind('\n').map(|v| v + 1).unwrap_or(0);
        if !source[line_start..pos].trim().is_empty() {
            cursor = pos + 7;
            continue;
        }
        let name_start = pos + 7;
        let name = read_ident(source, name_start)
            .ok_or_else(|| CompileError::Syntax("object name expected".into()))?;
        if !name
            .chars()
            .next()
            .map(|c| c.is_ascii_uppercase())
            .unwrap_or(false)
        {
            return Err(CompileError::Syntax(format!(
                "object `{name}` must start with an uppercase ASCII letter"
            )));
        }
        let after_name = name_start + name.len();
        let open = source[after_name..]
            .find('{')
            .map(|v| after_name + v)
            .ok_or_else(|| CompileError::Syntax(format!("object `{name}` has no body")))?;
        if !source[after_name..open].trim().is_empty() {
            return Err(CompileError::Syntax(format!(
                "object `{name}` expected `{{` after name"
            )));
        }
        let close = matching_brace(source, open)
            .ok_or_else(|| CompileError::Syntax(format!("object `{name}` body is unclosed")))?;
        let body = &source[open + 1..close];
        if body.contains("object ") {
            return Err(CompileError::Syntax(format!(
                "nested object declarations are not allowed inside `{name}`"
            )));
        }
        let mut members = Vec::new();
        let mut saw_model = false;
        for raw in body.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            if line.starts_with("route ") || line.starts_with("form ") {
                return Err(CompileError::Syntax(format!(
                    "object `{name}` keeps routes/forms top-level; found `{line}`"
                )));
            }
            if line == "model {" || line == "model{" {
                if saw_model {
                    return Err(CompileError::Syntax(format!(
                        "object `{name}` may contain only one model block"
                    )));
                }
                saw_model = true;
                continue;
            }
            for prefix in [
                "query fn ",
                "component fn ",
                "layout fn ",
                "page fn ",
                "action fn ",
            ] {
                if let Some(rest) = line.strip_prefix(prefix) {
                    let member = rest
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect::<String>();
                    if member.is_empty() || !is_identifier(&member) {
                        return Err(CompileError::Syntax(format!(
                            "object `{name}` has invalid member declaration near `{line}`"
                        )));
                    }
                    if members.iter().any(|v| v == &member) {
                        return Err(CompileError::Syntax(format!(
                            "object `{name}` duplicate member `{member}`"
                        )));
                    }
                    members.push(member);
                    break;
                }
            }
        }
        if !saw_model {
            return Err(CompileError::Syntax(format!(
                "object `{name}` requires exactly one `model {{ ... }}` block"
            )));
        }
        out.push(DomainObjectSpec {
            name,
            open,
            close,
            members,
        });
        cursor = close + 1;
    }
    Ok(out)
}

fn expand_domain_source(source: &str, specs: &[DomainObjectSpec]) -> Result<String, CompileError> {
    if specs.is_empty() {
        return Ok(source.to_string());
    }
    let mut out = String::with_capacity(source.len() + 256);
    let mut cursor = 0usize;
    for spec in specs {
        let object_pos = source[..spec.open]
            .rfind("object ")
            .ok_or_else(|| CompileError::Syntax("internal object expansion error".into()))?;
        out.push_str(&source[cursor..object_pos]);
        for ch in source[object_pos..spec.open + 1].chars() {
            out.push(if ch == '\n' { '\n' } else { ' ' });
        }
        let mut body = source[spec.open + 1..spec.close].to_string();
        if let Some(pos) = body.find("model {") {
            body.replace_range(pos..pos + 7, &format!("model {} {{", spec.name));
        } else if let Some(pos) = body.find("model{") {
            body.replace_range(pos..pos + 6, &format!("model {} {{", spec.name));
        }
        for member in &spec.members {
            for prefix in [
                "query fn ",
                "component fn ",
                "layout fn ",
                "page fn ",
                "action fn ",
            ] {
                let from = format!("{prefix}{member}");
                let to = format!("{prefix}{}__{member}", spec.name);
                body = body.replace(&from, &to);
            }
        }
        out.push_str(&body);
        out.push(' ');
        cursor = spec.close + 1;
    }
    out.push_str(&source[cursor..]);
    Ok(out)
}
