use crate::CompileError;

pub(crate) fn line_number(source: &str, offset: usize) -> usize {
    1 + source.as_bytes()[..offset.min(source.len())]
        .iter()
        .filter(|byte| **byte == b'\n')
        .count()
}

pub(crate) fn function_bounds(
    source: &str,
    start: usize,
    kind: &str,
) -> Result<(String, usize, usize, usize, usize), CompileError> {
    let name = read_ident(source, start)
        .ok_or_else(|| CompileError::Syntax(format!("{kind} name expected")))?;
    let sig_open = source[start + name.len()..]
        .find('(')
        .map(|value| start + name.len() + value)
        .ok_or_else(|| CompileError::Syntax(format!("{kind} `{name}` missing (")))?;
    let sig_close = matching_paren(source, sig_open)
        .ok_or_else(|| CompileError::Syntax(format!("{kind} `{name}` signature unclosed")))?;
    let body_open = source[sig_close + 1..]
        .find('{')
        .map(|value| sig_close + 1 + value)
        .ok_or_else(|| CompileError::Syntax(format!("{kind} `{name}` has no body")))?;
    let body_close = matching_brace(source, body_open)
        .ok_or_else(|| CompileError::Syntax(format!("{kind} `{name}` body unclosed")))?;
    Ok((name, sig_open, sig_close, body_open, body_close))
}

pub(crate) fn read_ident(source: &str, start: usize) -> Option<String> {
    let rest = &source[start..];
    let len = rest
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .count();
    (len > 0).then(|| rest[..len].into())
}

pub(crate) fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(ch) if ch == '_' || ch.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

pub(crate) fn matching_brace(source: &str, open: usize) -> Option<usize> {
    matching_delim(source, open, b'{', b'}')
}

pub(crate) fn matching_paren(source: &str, open: usize) -> Option<usize> {
    matching_delim(source, open, b'(', b')')
}

fn matching_delim(source: &str, open: usize, start: u8, end: u8) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (index, byte) in bytes.iter().copied().enumerate().skip(open) {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        if byte == b'"' {
            in_string = true;
            continue;
        }
        if byte == start {
            depth += 1;
        } else if byte == end {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

pub(crate) fn split_top_level(input: &str, separator: char) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '(' | '<' | '[' => depth += 1,
            ')' | '>' | ']' => depth -= 1,
            _ if ch == separator && depth == 0 => {
                out.push(&input[start..index]);
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    out.push(&input[start..]);
    out
}

pub(crate) fn find_statement_end(body: &str, start: usize) -> Result<usize, CompileError> {
    let bytes = body.as_bytes();
    let mut cursor = start;
    let mut in_string = false;
    let mut escaped = false;
    let mut paren_depth = 0i32;
    let mut bracket_depth = 0i32;
    let mut brace_depth = 0i32;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            cursor += 1;
            continue;
        }
        if byte == b'"' {
            in_string = true;
            cursor += 1;
            continue;
        }
        match byte {
            b'(' => paren_depth += 1,
            b')' => paren_depth -= 1,
            b'[' => bracket_depth += 1,
            b']' => bracket_depth -= 1,
            b'{' => brace_depth += 1,
            b'}' => brace_depth -= 1,
            b';' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                return Ok(cursor);
            }
            _ => {}
        }
        cursor += 1;
    }
    Err(CompileError::Syntax(
        "simple statement must end with `;`".into(),
    ))
}

pub(crate) fn skip_ws_and_comments(input: &str, mut cursor: usize) -> usize {
    let bytes = input.as_bytes();
    loop {
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if bytes.get(cursor) == Some(&b'/') && bytes.get(cursor + 1) == Some(&b'/') {
            cursor += 2;
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                cursor += 1;
            }
            continue;
        }
        return cursor;
    }
}

pub(crate) fn consume_return_tail(body: &str, mut cursor: usize) -> Result<usize, CompileError> {
    while cursor < body.len()
        && matches!(body.as_bytes()[cursor], b')' | b' ' | b'\t' | b'\r' | b'\n')
    {
        cursor += 1;
    }
    if body.as_bytes().get(cursor) != Some(&b';') {
        return Err(CompileError::Syntax(
            "return statement must end with `;`".into(),
        ));
    }
    Ok(cursor + 1)
}

pub(crate) fn consume_tail_result(body: &str, mut cursor: usize) -> Result<usize, CompileError> {
    while cursor < body.len()
        && matches!(body.as_bytes()[cursor], b')' | b' ' | b'\t' | b'\r' | b'\n')
    {
        cursor += 1;
    }
    if cursor == body.len() {
        return Ok(cursor);
    }
    Err(CompileError::Syntax(
        "tail Result expression must be the final expression and must not end with `;`".into(),
    ))
}

pub(crate) fn preview(value: &str) -> String {
    value
        .chars()
        .take(40)
        .collect::<String>()
        .replace('\n', " ")
}
