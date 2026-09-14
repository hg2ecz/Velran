use super::{CompileError, tokenize};
use crate::declarations;

pub(super) struct RouteDeclaration {
    pub(super) source: String,
    pub(super) line: usize,
}

pub(super) fn top_level_route_declarations(
    source: &str,
) -> Result<Vec<RouteDeclaration>, CompileError> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut active: Option<(usize, String)> = None;

    for (line_idx, raw) in source.lines().enumerate() {
        let trimmed = raw.trim_start();
        let is_top_level = depth == 0;
        let starts_decl = is_top_level && declarations::starts_top_level_declaration(trimmed);

        if let Some((start_line, buf)) = active.as_mut() {
            // A multiline route may legitimately continue with tokens that are also
            // top-level declaration prefixes, most notably `form`. Decide this from
            // the route grammar rather than indentation so formatting stays free-form.
            if starts_decl && !is_route_continuation_line(trimmed) {
                return Err(route_termination_error(
                    *start_line,
                    buf,
                    Some(line_idx + 1),
                )?);
            }
            if !buf.is_empty() {
                buf.push('\n');
            }
            buf.push_str(raw);
            if route_declaration_complete(buf)? {
                out.push(RouteDeclaration {
                    source: std::mem::take(buf),
                    line: *start_line,
                });
                active = None;
            }
        } else if is_top_level && trimmed.starts_with("route ") {
            let mut buf = raw.to_string();
            if route_declaration_complete(&buf)? {
                out.push(RouteDeclaration {
                    source: buf,
                    line: line_idx + 1,
                });
            } else {
                active = Some((line_idx + 1, std::mem::take(&mut buf)));
            }
        }

        depth = update_brace_depth(raw, depth)?;
    }

    if let Some((start_line, buf)) = active {
        return Err(route_termination_error(start_line, &buf, None)?);
    }
    Ok(out)
}

fn route_termination_error(
    start_line: usize,
    source: &str,
    next_decl_line: Option<usize>,
) -> Result<CompileError, CompileError> {
    let tokens = tokenize(source)?;
    let has_handler = tokens
        .iter()
        .position(|token| token == "=>")
        .is_some_and(|arrow| tokens.get(arrow + 1).is_some());
    if has_handler {
        return Ok(CompileError::Syntax(format!(
            "line {start_line}: route declaration must end with `;`"
        )));
    }
    Ok(CompileError::Syntax(match next_decl_line {
        Some(line) => format!("line {start_line}: incomplete route declaration before line {line}"),
        None => format!("line {start_line}: incomplete route declaration"),
    }))
}

fn is_route_continuation_line(trimmed: &str) -> bool {
    if trimmed.starts_with("=>") {
        return true;
    }
    if trimmed.starts_with("query ") && !trimmed.starts_with("query fn ") {
        return true;
    }
    if trimmed.starts_with("form ") {
        // Top-level named form declarations open a body with `{`; route form
        // bindings/schemas do not.
        return !trimmed.contains('{');
    }
    [
        "json ",
        "upload ",
        "multipart ",
        "validate ",
        "public",
        "auth ",
        "rate ",
        "budget ",
        "cache ",
        "invalidate ",
    ]
    .iter()
    .any(|prefix| trimmed.starts_with(prefix))
}

fn route_declaration_complete(source: &str) -> Result<bool, CompileError> {
    let tokens = tokenize(source)?;
    Ok(tokens
        .iter()
        .position(|token| token == "=>")
        .is_some_and(|arrow| {
            tokens.get(arrow + 1).is_some()
                && tokens.get(arrow + 2).map(String::as_str) == Some(";")
        }))
}

fn update_brace_depth(line: &str, mut depth: i32) -> Result<i32, CompileError> {
    let bytes = line.as_bytes();
    let mut i = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    while i < bytes.len() {
        let ch = bytes[i];
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
        if ch == b'/' && bytes.get(i + 1) == Some(&b'/') {
            break;
        }
        match ch {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth < 0 {
                    return Err(CompileError::Syntax(
                        "unexpected closing brace while scanning top-level declarations".into(),
                    ));
                }
            }
            _ => {}
        }
        i += 1;
    }
    Ok(depth)
}
