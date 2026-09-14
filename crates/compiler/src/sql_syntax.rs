use crate::CompileError;

pub(crate) fn first_sql_keyword(sql: &str) -> Option<String> {
    sql.split_whitespace()
        .next()
        .map(|value| value.to_ascii_uppercase())
}

pub(crate) fn scan_bind_names(sql: &str) -> Result<Vec<String>, CompileError> {
    let bytes = sql.as_bytes();
    let mut cursor = 0;
    let mut out = Vec::new();
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'\'' | b'"' | b'`' => {
                let quote = bytes[cursor];
                cursor += 1;
                while cursor < bytes.len() {
                    if bytes[cursor] == quote {
                        if cursor + 1 < bytes.len() && bytes[cursor + 1] == quote {
                            cursor += 2;
                            continue;
                        }
                        cursor += 1;
                        break;
                    }
                    cursor += 1;
                }
            }
            b'-' if bytes.get(cursor + 1) == Some(&b'-') => {
                cursor += 2;
                while cursor < bytes.len() && bytes[cursor] != b'\n' {
                    cursor += 1;
                }
            }
            b'/' if bytes.get(cursor + 1) == Some(&b'*') => {
                cursor += 2;
                while cursor + 1 < bytes.len()
                    && !(bytes[cursor] == b'*' && bytes[cursor + 1] == b'/')
                {
                    cursor += 1;
                }
                if cursor + 1 >= bytes.len() {
                    return Err(CompileError::UnsafeSql("unclosed SQL comment".into()));
                }
                cursor += 2;
            }
            b';' => {
                return Err(CompileError::UnsafeSql(
                    "query fn may contain exactly one statement; semicolon is forbidden".into(),
                ));
            }
            b':' if bytes.get(cursor + 1) == Some(&b':') => cursor += 2,
            b':' => {
                let start = cursor + 1;
                if !matches!(
                    bytes.get(start),
                    Some(b'_') | Some(b'a'..=b'z') | Some(b'A'..=b'Z')
                ) {
                    cursor += 1;
                    continue;
                }
                let mut end = start + 1;
                while end < bytes.len()
                    && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_')
                {
                    end += 1;
                }
                out.push(sql[start..end].into());
                cursor = end;
            }
            _ => cursor += 1,
        }
    }
    Ok(out)
}
