const TOP_LEVEL_DECLARATION_PREFIXES: &[&str] = &[
    "#[inline",
    "fn ",
    "type ",
    "enum ",
    "struct ",
    "impl ",
    "object ",
    "model ",
    "permission ",
    "critical ",
    "webhook ",
    "integration ",
    "production ",
    "security event ",
    "query fn ",
    "form ",
    "component fn ",
    "layout fn ",
    "page fn ",
    "action fn ",
    "route ",
];

pub(super) fn starts_top_level_declaration(line: &str) -> bool {
    let line = line.strip_prefix("pub ").unwrap_or(line);
    TOP_LEVEL_DECLARATION_PREFIXES
        .iter()
        .any(|prefix| line.starts_with(prefix))
}

pub(super) fn is_top_level_declaration_at(source: &str, pos: usize) -> bool {
    let line_start = source[..pos].rfind('\n').map(|v| v + 1).unwrap_or(0);
    let prefix = source[line_start..pos].trim();
    if !prefix.is_empty() && prefix != "pub" {
        return false;
    }

    let b = source.as_bytes();
    let mut i = 0usize;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    while i < pos {
        let ch = b[i];
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == b'\\' {
                escaped = true;
            } else if ch == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if ch == b'/' && b.get(i + 1) == Some(&b'/') {
            i += 2;
            while i < pos && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        match ch {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => depth = (depth - 1).max(0),
            _ => {}
        }
        i += 1;
    }
    depth == 0 && !in_string
}

#[cfg(test)]
mod tests {
    use super::starts_top_level_declaration;

    #[test]
    fn rust_struct_is_a_top_level_declaration() {
        assert!(starts_top_level_declaration("struct SearchParams {"));
    }
}
