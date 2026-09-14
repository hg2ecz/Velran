use crate::diagnostics::CompileError;

pub(super) const MAX_DB_LIST_ROWS: u64 = 1000;

pub(super) fn require_bounded_list_query(name: &str, sql: &str) -> Result<(), CompileError> {
    let tokens = sql_tokens(sql);
    let mut limits = tokens
        .windows(2)
        .filter(|pair| pair[0].eq_ignore_ascii_case("limit"))
        .map(|pair| pair[1].parse::<u64>());
    let Some(limit) = limits.next() else {
        return Err(CompileError::security(
            "SEC-RESOURCE-001",
            format!("List query `{name}` requires a literal LIMIT"),
            Some(format!(
                "add `LIMIT n` with n in 1..={MAX_DB_LIST_ROWS}; unbounded DB result collections are forbidden"
            )),
        ));
    };
    if limits.next().is_some() {
        return Err(CompileError::security(
            "SEC-RESOURCE-001",
            format!("List query `{name}` has ambiguous LIMIT clauses"),
            Some("use exactly one literal LIMIT clause".into()),
        ));
    }
    match limit {
        Ok(value) if (1..=MAX_DB_LIST_ROWS).contains(&value) => Ok(()),
        _ => Err(CompileError::security(
            "SEC-RESOURCE-001",
            format!("List query `{name}` LIMIT must be a literal in 1..={MAX_DB_LIST_ROWS}"),
            Some("use a compile-time literal row bound; parameterized or oversized LIMIT values are forbidden".into()),
        )),
    }
}

fn sql_tokens(sql: &str) -> Vec<String> {
    let bytes = sql.as_bytes();
    let mut cursor = 0usize;
    let mut tokens = Vec::new();
    while cursor < bytes.len() {
        match bytes[cursor] {
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
                cursor = (cursor + 2).min(bytes.len());
            }
            b'\'' | b'"' | b'`' => {
                let quote = bytes[cursor];
                cursor += 1;
                while cursor < bytes.len() {
                    if bytes[cursor] == quote {
                        if bytes.get(cursor + 1) == Some(&quote) {
                            cursor += 2;
                            continue;
                        }
                        cursor += 1;
                        break;
                    }
                    cursor += 1;
                }
            }
            b if b.is_ascii_alphabetic() || b == b'_' => {
                let start = cursor;
                cursor += 1;
                while cursor < bytes.len()
                    && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
                {
                    cursor += 1;
                }
                tokens.push(sql[start..cursor].to_string());
            }
            b if b.is_ascii_digit() => {
                let start = cursor;
                cursor += 1;
                while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                    cursor += 1;
                }
                tokens.push(sql[start..cursor].to_string());
            }
            b':' | b'?' | b'$' => {
                tokens.push((bytes[cursor] as char).to_string());
                cursor += 1;
            }
            _ => cursor += 1,
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_literal_limit_and_ignores_comments_and_strings() {
        assert!(require_bounded_list_query("items", "SELECT id FROM items LIMIT 100").is_ok());
        assert!(
            require_bounded_list_query(
                "items",
                "SELECT 'LIMIT 9999' FROM items /* LIMIT 9999 */ LIMIT 10"
            )
            .is_ok()
        );
    }

    #[test]
    fn rejects_missing_parameterized_and_oversized_limit() {
        for sql in [
            "SELECT id FROM items",
            "SELECT id FROM items LIMIT :limit",
            "SELECT id FROM items LIMIT 1001",
            "SELECT id FROM items LIMIT 0",
        ] {
            let error = require_bounded_list_query("items", sql).unwrap_err();
            assert!(error.to_string().contains("SEC-RESOURCE-001"));
        }
    }
}
