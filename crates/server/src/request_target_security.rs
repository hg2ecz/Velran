/// Strict origin-form request-target validation for the server boundary.
///
/// Security invariants:
/// - only a rooted origin-form path is accepted;
/// - path segments may not be `.` / `..`, including percent-encoded forms;
/// - encoded `/` and `\\` are rejected so downstream routing cannot see a
///   different path structure than the HTTP parser;
/// - malformed percent escapes and encoded control/NUL bytes fail closed.
pub(super) fn valid_origin_form_target(target: &str) -> bool {
    if target.is_empty()
        || !target.starts_with('/')
        || target.starts_with("//")
        || target.contains('#')
        || target.contains('\\')
        || target.bytes().any(|b| b <= 0x20 || b >= 0x7f)
    {
        return false;
    }

    let path = target
        .split_once('?')
        .map(|(path, _)| path)
        .unwrap_or(target);
    if path.is_empty() {
        return false;
    }

    for segment in path.split('/') {
        if segment.is_empty() {
            continue;
        }
        let Some(decoded) = decode_path_segment(segment) else {
            return false;
        };
        if decoded == b"." || decoded == b".." || decoded.first() == Some(&b'.') {
            return false;
        }
    }
    true
}

fn decode_path_segment(segment: &str) -> Option<Vec<u8>> {
    let bytes = segment.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                if index + 2 >= bytes.len() {
                    return None;
                }
                let hi = hex(bytes[index + 1])?;
                let lo = hex(bytes[index + 2])?;
                let value = (hi << 4) | lo;
                if value <= 0x20 || value == 0x7f || matches!(value, b'/' | b'\\') {
                    return None;
                }
                out.push(value.to_ascii_lowercase());
                index += 3;
            }
            value => {
                out.push(value.to_ascii_lowercase());
                index += 1;
            }
        }
    }
    Some(out)
}

fn hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::valid_origin_form_target;

    #[test]
    fn rejects_literal_and_encoded_dot_segments() {
        for target in [
            "/./admin",
            "/../admin",
            "/%2e/admin",
            "/%2E%2E/admin",
            "/.hidden/file",
            "/%2ehidden/file",
        ] {
            assert!(!valid_origin_form_target(target), "{target}");
        }
    }

    #[test]
    fn rejects_encoded_separators_and_bad_percent_escapes() {
        for target in ["/a%2fb", "/a%5Cb", "/bad%", "/bad%2", "/bad%zz"] {
            assert!(!valid_origin_form_target(target), "{target}");
        }
    }

    #[test]
    fn accepts_normal_origin_form_targets() {
        for target in ["/", "/users/42", "/search?q=a%20b", "/assets/app.js"] {
            assert!(valid_origin_form_target(target), "{target}");
        }
    }
}
