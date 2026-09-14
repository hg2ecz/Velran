pub(super) fn sql_has_tenant_guard(keyword: &str, sql: &str, field: &str) -> bool {
    match keyword {
        "INSERT" => insert_pairs_field_with_bind(sql, field),
        "SELECT" | "UPDATE" | "DELETE" => where_has_field_bind_equality(sql, field),
        _ => false,
    }
}

fn where_has_field_bind_equality(sql: &str, field: &str) -> bool {
    let lower = sql.to_ascii_lowercase();
    let Some(where_pos) = lower.find("where") else {
        return false;
    };
    let tokens = tenant_predicate_tokens(&lower[where_pos + 5..]);
    let field = field.to_ascii_lowercase();
    let bind = format!(":{field}");
    for index in 0..tokens.len() {
        if tokens[index] != "=" {
            continue;
        }
        let left = operand_before(&tokens, index);
        let right = operand_after(&tokens, index);
        if matches_field(left, &field) && right == Some(bind.as_str()) {
            return true;
        }
        if left == Some(bind.as_str()) && matches_field(right, &field) {
            return true;
        }
    }
    false
}

fn tenant_predicate_tokens(sql: &str) -> Vec<String> {
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
                        cursor += 1;
                        break;
                    }
                    cursor += 1;
                }
            }
            b':' if bytes
                .get(cursor + 1)
                .is_some_and(|next| next.is_ascii_alphabetic() || *next == b'_') =>
            {
                let start = cursor;
                cursor += 2;
                while cursor < bytes.len()
                    && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
                {
                    cursor += 1;
                }
                tokens.push(sql[start..cursor].to_string());
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
            b'=' | b'.' => {
                tokens.push((bytes[cursor] as char).to_string());
                cursor += 1;
            }
            _ => cursor += 1,
        }
    }
    tokens
}

fn operand_before(tokens: &[String], equals: usize) -> Option<&str> {
    if equals == 0 {
        return None;
    }
    tokens.get(equals - 1).map(String::as_str)
}

fn operand_after(tokens: &[String], equals: usize) -> Option<&str> {
    tokens.get(equals + 1).map(String::as_str)
}

fn matches_field(value: Option<&str>, field: &str) -> bool {
    value == Some(field)
}

fn insert_pairs_field_with_bind(sql: &str, field: &str) -> bool {
    let lower = sql.to_ascii_lowercase();
    let Some(values_pos) = lower.find("values") else {
        return false;
    };
    let before = &lower[..values_pos];
    let after = &lower[values_pos + 6..];
    let Some(columns_open) = before.rfind('(') else {
        return false;
    };
    let Some(columns_close_rel) = before[columns_open + 1..].find(')') else {
        return false;
    };
    let columns_close = columns_open + 1 + columns_close_rel;
    let Some(values_open) = after.find('(') else {
        return false;
    };
    let Some(values_close_rel) = after[values_open + 1..].find(')') else {
        return false;
    };
    let values_close = values_open + 1 + values_close_rel;
    let columns: Vec<_> = before[columns_open + 1..columns_close]
        .split(',')
        .map(|v| v.trim())
        .collect();
    let values: Vec<_> = after[values_open + 1..values_close]
        .split(',')
        .map(|v| v.trim())
        .collect();
    let field_lower = field.to_ascii_lowercase();
    let bind = format!(":{field_lower}");
    columns.iter().zip(values.iter()).any(|(column, value)| {
        column.rsplit('.').next() == Some(field_lower.as_str()) && *value == bind
    })
}
