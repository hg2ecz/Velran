use crate::diagnostics::CompileError;

const LEGACY_CALLABLE_PREFIXES: [(&str, &str); 5] = [
    ("page fn ", "#[page] fn"),
    ("action fn ", "#[action] fn"),
    ("query fn ", "#[query] fn"),
    ("component fn ", "#[component] fn"),
    ("layout fn ", "#[layout] fn"),
];

const FORBIDDEN_PUBLIC_FRAMEWORK_PREFIXES: &[&str] = &[
    "pub model ",
    "pub object ",
    "pub form ",
    "pub permission ",
    "pub critical ",
    "pub webhook ",
    "pub integration ",
    "pub production ",
    "pub security event ",
    "pub route ",
    "pub page fn ",
    "pub action fn ",
    "pub query fn ",
    "pub component fn ",
    "pub layout fn ",
];

const CALLABLE_ATTRS: [(&str, &str); 5] = [
    ("#[page]", "page"),
    ("#[action]", "action"),
    ("#[query]", "query"),
    ("#[component]", "component"),
    ("#[layout]", "layout"),
];

pub(crate) fn normalize(source: &str) -> Result<String, CompileError> {
    let mut out = String::with_capacity(source.len() + 64);
    let mut pending: Option<&'static str> = None;

    for (line_index, raw) in source.split_inclusive('\n').enumerate() {
        let (line, newline) = raw
            .strip_suffix('\n')
            .map_or((raw, ""), |line| (line, "\n"));
        let trimmed = line.trim();

        if let Some(prefix) = FORBIDDEN_PUBLIC_FRAMEWORK_PREFIXES
            .iter()
            .find(|prefix| trimmed.starts_with(**prefix))
        {
            return Err(CompileError::Syntax(format!(
                "line {}: `{}` is not a library visibility boundary; web/framework authority remains route/capability-owned",
                line_index + 1,
                prefix.trim()
            )));
        }

        if let Some((legacy, replacement)) = LEGACY_CALLABLE_PREFIXES
            .iter()
            .find(|(legacy, _)| trimmed.starts_with(*legacy))
        {
            return Err(CompileError::Syntax(format!(
                "line {}: legacy callable syntax `{}` is not supported; use `{replacement}`",
                line_index + 1,
                legacy.trim()
            )));
        }

        if trimmed.starts_with("#[")
            && !CALLABLE_ATTRS
                .iter()
                .any(|(attr, _)| trimmed == *attr || trimmed.starts_with(&format!("{attr} fn ")))
            && !matches!(
                trimmed,
                "#[inline]" | "#[inline(always)]" | "#[inline(never)]"
            )
            && !trimmed.starts_with("#[inline] fn ")
            && !trimmed.starts_with("#[inline(always)] fn ")
            && !trimmed.starts_with("#[inline(never)] fn ")
        {
            return Err(CompileError::Syntax(format!(
                "line {}: unsupported Rust attribute `{}`; only Velran callable attributes and #[inline(...)] are allowed",
                line_index + 1,
                trimmed
            )));
        }

        if let Some((_, kind)) = CALLABLE_ATTRS.iter().find(|(attr, _)| trimmed == *attr) {
            if pending.replace(*kind).is_some() {
                return Err(CompileError::Syntax(
                    "callable attributes may not be stacked before a function".into(),
                ));
            }
            // Preserve the original line number for diagnostics.
            out.push_str(&" ".repeat(line.len()));
            out.push_str(newline);
            continue;
        }

        if pending.is_none() {
            if let Some((attr, kind)) = CALLABLE_ATTRS
                .iter()
                .find(|(attr, _)| trimmed.starts_with(&format!("{attr} fn ")))
            {
                let indent_len = line.len() - line.trim_start().len();
                out.push_str(&line[..indent_len]);
                out.push_str(kind);
                out.push_str(" fn ");
                let rest = line.trim_start().strip_prefix(attr).unwrap().trim_start();
                out.push_str(rest.strip_prefix("fn ").unwrap());
                out.push_str(newline);
                continue;
            }
        }

        if let Some(kind) = pending.take() {
            let trimmed_start = line.trim_start();
            if trimmed_start.starts_with("//") || trimmed_start.is_empty() {
                pending = Some(kind);
                out.push_str(line);
                out.push_str(newline);
                continue;
            }
            let Some(rest) = trimmed_start.strip_prefix("fn ") else {
                return Err(CompileError::Syntax(format!(
                    "#[{kind}] must be followed by a function declaration"
                )));
            };
            let indent_len = line.len() - trimmed_start.len();
            out.push_str(&line[..indent_len]);
            out.push_str(kind);
            out.push_str(" fn ");
            out.push_str(rest);
            out.push_str(newline);
            continue;
        }

        out.push_str(line);
        out.push_str(newline);
    }

    if let Some(kind) = pending {
        return Err(CompileError::Syntax(format!(
            "#[{kind}] must be followed by a function declaration"
        )));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn normalizes_callable_attributes_without_changing_line_count() {
        let source = "#[page]\nfn home(ctx: PageContext) -> Result<Html, PageError> {\n}\n";
        let normalized = normalize(source).unwrap();
        assert_eq!(normalized.lines().count(), source.lines().count());
        assert!(normalized.contains("page fn home"));
    }

    #[test]
    fn supports_same_line_attribute_form() {
        let source = "#[action] fn save(ctx: ActionContext) -> Result<Json, PageError> { }\n";
        assert!(normalize(source).unwrap().contains("action fn save"));
    }

    #[test]
    fn rejects_legacy_callable_keywords() {
        let err = normalize("page fn home(ctx: PageContext) -> Result<Html, PageError> { }\n")
            .unwrap_err();
        assert!(err.to_string().contains("#[page] fn"));
    }

    #[test]
    fn rejects_pub_on_framework_authority_declarations() {
        for source in [
            "pub model User { id: i64 }\n",
            "pub route home GET \"/\" public => home;\n",
        ] {
            assert!(normalize(source).is_err(), "unexpectedly allowed: {source}");
        }
    }

    #[test]
    fn rejects_linker_and_abi_attributes() {
        for attr in [
            "#[no_mangle]",
            "#[export_name = \"pwn\"]",
            "#[link_section = \".text\"]",
            "#[repr(C)]",
        ] {
            let source = format!("{attr}\nfn helper(real: &mut [f32; 4]) {{ }}\n");
            assert!(
                normalize(&source).is_err(),
                "attribute unexpectedly allowed: {attr}"
            );
        }
    }
}

#[cfg(test)]
mod line_diagnostic_tests {
    use super::normalize;

    #[test]
    fn legacy_callable_error_reports_source_line() {
        let source = "// header\npage fn home(ctx: PageContext) -> Result<Html, PageError> { }\n";
        let err = normalize(source).unwrap_err();
        assert!(err.to_string().contains("line 2:"));
    }
}
