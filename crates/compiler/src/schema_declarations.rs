use crate::declarations;
use crate::diagnostics::CompileError;
use crate::module_namespace::qualify;
use crate::source_syntax::{is_identifier, matching_brace, read_ident};
use crate::type_resolution::resolve_annotated_value_type;
use language_core::{FormField, FunctionParam, JsonSchema, Model, Program, ValueType};

pub(super) fn parse_enums(
    source: &str,
    namespace: &str,
    p: &mut Program,
) -> Result<(), CompileError> {
    let mut off = 0usize;
    while let Some(rel) = source[off..].find("enum ") {
        let pos = off + rel;
        if !declarations::is_top_level_declaration_at(source, pos) {
            off = pos + 5;
            continue;
        }
        if pos > 0 {
            let prev = source.as_bytes()[pos - 1];
            if prev.is_ascii_alphanumeric() || prev == b'_' {
                off = pos + 5;
                continue;
            }
        }
        let start = pos + 5;
        let name = read_ident(source, start)
            .ok_or_else(|| CompileError::Syntax("enum name expected".into()))?;
        if !name
            .chars()
            .next()
            .map(|c| c.is_ascii_uppercase())
            .unwrap_or(false)
        {
            return Err(CompileError::Syntax(format!(
                "enum `{name}` must start with an uppercase ASCII letter"
            )));
        }
        let symbol_name = qualify(namespace, &name);
        if p.enum_by_name(&symbol_name).is_some() {
            return Err(CompileError::Syntax(format!("duplicate enum `{name}`")));
        }
        if p.enums.len() >= u16::MAX as usize {
            return Err(CompileError::Syntax("too many enum declarations".into()));
        }
        let after = start + name.len();
        let open = source[after..]
            .find('{')
            .map(|v| after + v)
            .ok_or_else(|| CompileError::Syntax(format!("enum `{name}` has no body")))?;
        let close = matching_brace(source, open)
            .ok_or_else(|| CompileError::Syntax(format!("enum `{name}` body is unclosed")))?;
        let mut variants = Vec::new();
        for line in source[open + 1..close].lines() {
            let clean = line.split_once("//").map(|(a, _)| a).unwrap_or(line);
            for raw in clean.split(|c: char| c == ',' || c.is_whitespace()) {
                let v = raw.trim();
                if v.is_empty() {
                    continue;
                }
                if !is_identifier(v)
                    || !v
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_uppercase())
                        .unwrap_or(false)
                {
                    return Err(CompileError::Syntax(format!(
                        "enum `{name}` invalid variant `{v}`"
                    )));
                }
                if variants.iter().any(|x: &String| x == v) {
                    return Err(CompileError::Syntax(format!(
                        "enum `{name}` duplicate variant `{v}`"
                    )));
                }
                variants.push(v.to_string());
            }
        }
        if variants.is_empty() {
            return Err(CompileError::Syntax(format!(
                "enum `{name}` requires at least one variant"
            )));
        }
        if variants.len() > 256 {
            return Err(CompileError::Syntax(format!(
                "enum `{name}` has too many variants (max 256)"
            )));
        }
        crate::visibility::register(
            p,
            symbol_name.clone(),
            crate::visibility::declaration_visibility(source, pos),
        );
        p.enums.push(language_core::EnumDef {
            name: symbol_name,
            variants,
        });
        off = close + 1;
    }
    Ok(())
}

pub(super) fn parse_json_structs(
    source: &str,
    namespace: &str,
    p: &mut Program,
) -> Result<(), CompileError> {
    let mut off = 0usize;
    while let Some(rel) = source[off..].find("struct ") {
        let keyword = off + rel;
        if !declarations::is_top_level_declaration_at(source, keyword) {
            off = keyword + 7;
            continue;
        }
        let start = keyword + 7;
        let name = read_ident(source, start)
            .ok_or_else(|| CompileError::Syntax("struct name expected".into()))?;
        if !name
            .chars()
            .next()
            .map(|c| c.is_ascii_uppercase())
            .unwrap_or(false)
        {
            return Err(CompileError::Syntax(format!(
                "struct `{name}` must start with an uppercase ASCII letter"
            )));
        }
        let symbol_name = qualify(namespace, &name);
        if p.json_schema(&symbol_name).is_some() {
            return Err(CompileError::Syntax(format!("duplicate struct `{name}`")));
        }
        let after = start + name.len();
        let open = source[after..]
            .find('{')
            .map(|v| after + v)
            .ok_or_else(|| CompileError::Syntax(format!("struct `{name}` has no body")))?;
        let close = matching_brace(source, open)
            .ok_or_else(|| CompileError::Syntax(format!("struct `{name}` body is unclosed")))?;
        let mut fields = Vec::new();
        let mut field_visibilities = Vec::new();
        for raw in crate::source_syntax::split_top_level(&source[open + 1..close], ',')
            .into_iter()
            .flat_map(|part| part.lines())
            .map(str::trim)
            .filter(|v| !v.is_empty() && !v.starts_with("//"))
        {
            let raw = raw.trim();
            let (visibility, raw) = crate::visibility::strip_member_visibility(raw);
            let (field, ty) = raw.split_once(':').ok_or_else(|| {
                CompileError::Syntax(format!(
                    "struct `{name}` field `{raw}` expected `name: Type`"
                ))
            })?;
            let field = field.trim();
            let ty = ty.trim();
            if !is_identifier(field) {
                return Err(CompileError::Syntax(format!(
                    "struct `{name}` invalid field `{field}`"
                )));
            }
            let resolved = resolve_annotated_value_type(ty, namespace, p)
                .filter(|v| v.sensitivity == language_core::DataSensitivity::Public)
                .filter(|v| !matches!(v.value_type, ValueType::F32Array | ValueType::StringDict))
                .ok_or_else(|| {
                    CompileError::Syntax(format!(
                        "struct `{name}` unsupported JSON field type `{ty}`"
                    ))
                })?;
            if fields.iter().any(|f: &FormField| f.name == field) {
                return Err(CompileError::Syntax(format!(
                    "struct `{name}` duplicate field `{field}`"
                )));
            }
            fields.push(FormField {
                name: field.into(),
                ty: resolved.value_type,
            });
            field_visibilities.push((field.to_string(), visibility));
        }
        if fields.is_empty() {
            return Err(CompileError::Syntax(format!(
                "struct `{name}` requires at least one field"
            )));
        }
        if fields.len() > 1024 {
            return Err(CompileError::Syntax(format!(
                "struct `{name}` has too many fields (max 1024)"
            )));
        }
        crate::visibility::register(
            p,
            symbol_name.clone(),
            crate::visibility::declaration_visibility(source, keyword),
        );
        for (field, visibility) in field_visibilities {
            crate::visibility::register_member(p, &symbol_name, &field, visibility);
        }
        p.json_schemas.push(JsonSchema {
            name: symbol_name,
            fields,
        });
        off = close + 1;
    }
    Ok(())
}

mod forms;

pub(super) fn parse_form_schemas(
    source: &str,
    namespace: &str,
    p: &mut Program,
) -> Result<(), CompileError> {
    forms::parse_form_schemas(source, namespace, p)
}

pub(super) fn parse_models(
    source: &str,
    namespace: &str,
    p: &mut Program,
) -> Result<(), CompileError> {
    let mut off = 0;
    while let Some(rel) = source[off..].find("model ") {
        let keyword = off + rel;
        if !declarations::is_top_level_declaration_at(source, keyword) {
            off = keyword + 6;
            continue;
        }
        let start = keyword + 6;
        let name = read_ident(source, start)
            .ok_or_else(|| CompileError::Syntax("model name expected".into()))?;
        let symbol_name = qualify(namespace, &name);
        if p.models.iter().any(|m| m.name == symbol_name) {
            return Err(CompileError::Syntax(format!("duplicate model `{name}`")));
        }
        let after_name = start + name.len();
        let brace = source[after_name..]
            .find('{')
            .map(|v| after_name + v)
            .ok_or_else(|| CompileError::Syntax(format!("model `{name}` has no body")))?;
        let header = source[after_name..brace].trim();
        let tenant_field = if header.is_empty() {
            None
        } else if let Some(field) = header.strip_prefix("scoped by ") {
            let field = field.trim();
            if !is_identifier(field) {
                return Err(CompileError::Syntax(format!(
                    "model `{name}` tenant scope must use `scoped by <field>`"
                )));
            }
            Some(field.to_string())
        } else {
            return Err(CompileError::Syntax(format!(
                "model `{name}` expected `{{` or `scoped by <field> {{`"
            )));
        };
        let close = matching_brace(source, brace)
            .ok_or_else(|| CompileError::Syntax(format!("model `{name}` body is unclosed")))?;
        let mut fields = Vec::new();
        for raw in source[brace + 1..close]
            .lines()
            .map(str::trim)
            .filter(|v| !v.is_empty() && !v.starts_with("//"))
        {
            let raw = raw.trim_end_matches(',').trim();
            let (_visibility, raw) = crate::visibility::strip_member_visibility(raw);
            let (field, ty) = raw.split_once(':').ok_or_else(|| {
                CompileError::Syntax(format!(
                    "model `{name}` field `{raw}` expected `name: Type`"
                ))
            })?;
            let field = field.trim();
            let ty = ty.trim();
            if !is_identifier(field) {
                return Err(CompileError::Syntax(format!(
                    "model `{name}` invalid field `{field}`"
                )));
            }
            let annotated = resolve_annotated_value_type(ty, namespace, p)
                .filter(|resolved| {
                    !matches!(
                        resolved.value_type,
                        ValueType::Upload
                            | ValueType::F32Array
                            | ValueType::StringList
                            | ValueType::StringDict
                    )
                })
                .ok_or_else(|| {
                    CompileError::Syntax(format!("model `{name}` unsupported field type `{ty}`"))
                })?;
            if fields.iter().any(|f: &FunctionParam| f.name == field) {
                return Err(CompileError::Syntax(format!(
                    "model `{name}` duplicate field `{field}`"
                )));
            }
            fields.push(FunctionParam {
                name: field.into(),
                ty: annotated.value_type,
                sensitivity: annotated.sensitivity,
            });
        }
        if fields.is_empty() {
            return Err(CompileError::Syntax(format!(
                "model `{name}` must have at least one field"
            )));
        }
        if let Some(scope_field) = tenant_field.as_deref() {
            let scope = fields.iter().find(|field| field.name == scope_field).ok_or_else(|| {
                CompileError::security(
                    "SEC-A01-030",
                    format!("tenant-scoped model `{name}` references missing field `{scope_field}`"),
                    Some("declare the tenant field on the model and use a String-represented nominal domain type such as OrganizationId".into()),
                )
            })?;
            if p.representation_type(scope.ty) != Some(ValueType::String) {
                return Err(CompileError::security(
                    "SEC-A01-031",
                    format!("tenant field `{name}.{scope_field}` must have String representation"),
                    Some("use a nominal String domain type for tenant identifiers so it can be matched against authenticated membership claims".into()),
                ));
            }
            if scope.sensitivity != language_core::DataSensitivity::Public {
                return Err(CompileError::security(
                    "SEC-A01-040",
                    format!("tenant field `{name}.{scope_field}` must be public identity metadata"),
                    Some("tenant scope identifiers are authorization selectors, not secrets; keep sensitive tenant data in separate fields".into()),
                ));
            }
        }
        p.models.push(Model {
            name: symbol_name,
            fields,
            tenant_field,
        });
        off = close + 1;
    }
    Ok(())
}
