use crate::source_syntax::is_identifier;

/// Returns the offset immediately after a Rust-like local declaration
/// prefix. Both `let x = ...` and `let mut x = ...` lower into the existing
/// verified local-declaration IR. Mutability becomes a frontend contract; the
/// runtime does not gain a second assignment mechanism.
pub(crate) fn let_binding_start(source: &str, cursor: usize) -> Option<usize> {
    let tail = source.get(cursor..)?;
    if tail.starts_with("let mut ") {
        Some(cursor + "let mut ".len())
    } else if tail.starts_with("let ") {
        Some(cursor + "let ".len())
    } else {
        None
    }
}

/// Identifies a Rust-like direct assignment statement at `cursor` without
/// confusing comparison operators with assignment. The caller still performs
/// the existing type, collection and security validation before creating IR.
pub(crate) fn direct_assignment_start(source: &str, cursor: usize) -> bool {
    let Some(tail) = source.get(cursor..) else {
        return false;
    };
    let Some(end) = tail.find(';') else {
        return false;
    };
    let stmt = tail[..end].trim();
    if stmt.is_empty() || stmt.starts_with("let ") {
        return false;
    }
    if normalize_compound_assignment(stmt).is_some() {
        return true;
    }
    let Some((lhs, _rhs)) = split_plain_assignment(stmt) else {
        return false;
    };
    let lhs = lhs.trim();
    if is_identifier(lhs) {
        return true;
    }
    if let Some(open) = lhs.find('[') {
        let base = lhs[..open].trim();
        return is_identifier(base) && lhs.ends_with(']');
    }
    false
}

pub(crate) fn assignment_text(source: &str, cursor: usize, end: usize) -> String {
    let raw = source[cursor..end].trim().trim_end_matches(';').trim();
    normalize_compound_assignment(raw).unwrap_or_else(|| raw.to_string())
}

fn normalize_compound_assignment(input: &str) -> Option<String> {
    for (needle, op) in [
        ("+=", "+"),
        ("-=", "-"),
        ("*=", "*"),
        ("/=", "/"),
        ("%=", "%"),
    ] {
        if let Some((lhs, rhs)) = input.split_once(needle) {
            let lhs = lhs.trim();
            let rhs = rhs.trim();
            if is_identifier(lhs) {
                return Some(format!("{lhs} = {lhs} {op} ({rhs})"));
            }
        }
    }
    None
}

fn split_plain_assignment(input: &str) -> Option<(&str, &str)> {
    let bytes = input.as_bytes();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (idx, &b) in bytes.iter().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match b {
                b'\\' => escaped = true,
                b'"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'=' if depth == 0 => {
                let prev = idx.checked_sub(1).and_then(|i| bytes.get(i)).copied();
                let next = bytes.get(idx + 1).copied();
                if matches!(prev, Some(b'=' | b'!' | b'<' | b'>')) || next == Some(b'=') {
                    continue;
                }
                return Some((&input[..idx], &input[idx + 1..]));
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_rust_like_let() {
        assert_eq!(let_binding_start("let x = 1;", 0), Some(4));
        assert_eq!(let_binding_start("let mut x = 1;", 0), Some(8));
    }

    #[test]
    fn assignment_detection_rejects_comparison() {
        assert!(direct_assignment_start("x = x + 1;", 0));
        assert!(direct_assignment_start("a[i] = 1.0;", 0));
        assert!(direct_assignment_start("x += 1;", 0));
        assert_eq!(assignment_text("x += 1;", 0, 6), "x = x + (1)");
        assert!(!direct_assignment_start("x == y;", 0));
        assert!(!direct_assignment_start("x <= y;", 0));
    }
}
