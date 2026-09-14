pub(super) const SOURCE: &str = r#"
fn bytes<'a>(raw: &'a VelranInputValue, max: usize) -> Option<&'a [u8]> {
    let len = usize::try_from(raw.len).ok()?; if len > max { return None; }
    if len == 0 { return Some(&[]); } if raw.data.is_null() { return None; }
    Some(unsafe { std::slice::from_raw_parts(raw.data, len) })
}

fn text<'a>(raw: &'a VelranInputValue, max: usize) -> Option<&'a str> {
    if raw.payload != 0 { return None; }
    let len = usize::try_from(raw.len).ok()?;
    if len > max { return None; }
    let bytes = if len == 0 { &[][..] } else {
        if raw.data.is_null() { return None; }
        unsafe { std::slice::from_raw_parts(raw.data, len) }
    };
    std::str::from_utf8(bytes).ok()
}

fn canonical_slug(value: &str) -> bool {
    if value.is_empty() || value.len() > 160 { return false; }
    let mut previous_hyphen = false;
    for (index, byte) in value.bytes().enumerate() {
        if byte == b'-' {
            if index == 0 || index + 1 == value.len() || previous_hyphen { return false; }
            previous_hyphen = true;
        } else if byte.is_ascii_lowercase() || byte.is_ascii_digit() {
            previous_hyphen = false;
        } else { return false; }
    }
    true
}

fn canonical_email(value: &str) -> bool {
    if value.is_empty() || value.len() > 254 || !value.is_ascii() || value.bytes().any(|b| b <= 0x20 || b == 0x7f) { return false; }
    let Some((local, domain)) = value.rsplit_once('@') else { return false; };
    if local.is_empty() || local.len() > 64 || domain.is_empty() || domain.len() > 253 || local.contains('@') || local.starts_with('.') || local.ends_with('.') || local.contains("..") { return false; }
    if !local.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.'|b'!'|b'#'|b'$'|b'%'|b'&'|b'\''|b'*'|b'+'|b'-'|b'/'|b'='|b'?'|b'^'|b'_'|b'`'|b'{'|b'|'|b'}'|b'~')) { return false; }
    if domain.bytes().any(|b| b.is_ascii_uppercase()) { return false; }
    domain.split('.').all(|label| !label.is_empty() && label.len() <= 63 && !label.starts_with('-') && !label.ends_with('-') && label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
}

fn safe_normalized_url(value: &str) -> bool {
    if value.is_empty() || value.len() > 2048 || value.bytes().any(|b| b <= 0x20 || b == 0x7f) { return false; }
    let rest = if let Some(v) = value.strip_prefix("http://") { v } else if let Some(v) = value.strip_prefix("https://") { v } else { return false; };
    let authority = rest.split(|c| matches!(c, '/' | '?' | '#')).next().unwrap_or("");
    !authority.is_empty() && !authority.contains('@')
}
"#;
