pub(super) fn is_canonical_slug(raw: &str) -> bool {
    if raw.is_empty() || raw.len() > 160 {
        return false;
    }
    let mut prev_hyphen = false;
    for (i, b) in raw.bytes().enumerate() {
        if b == b'-' {
            if i == 0 || i + 1 == raw.len() || prev_hyphen {
                return false;
            }
            prev_hyphen = true;
        } else if b.is_ascii_lowercase() || b.is_ascii_digit() {
            prev_hyphen = false;
        } else {
            return false;
        }
    }
    true
}

pub(super) fn normalize_email(raw: &str) -> Option<String> {
    if raw.is_empty()
        || raw.len() > 254
        || !raw.is_ascii()
        || raw.bytes().any(|b| b <= 0x20 || b == 0x7f)
    {
        return None;
    }
    let (local, domain) = raw.rsplit_once('@')?;
    if local.is_empty()
        || local.len() > 64
        || domain.is_empty()
        || domain.len() > 253
        || local.contains('@')
    {
        return None;
    }
    if local.starts_with('.') || local.ends_with('.') || local.contains("..") {
        return None;
    }
    if !local.bytes().all(|b| {
        b.is_ascii_alphanumeric()
            || matches!(
                b,
                b'.' | b'!'
                    | b'#'
                    | b'$'
                    | b'%'
                    | b'&'
                    | b'\''
                    | b'*'
                    | b'+'
                    | b'-'
                    | b'/'
                    | b'='
                    | b'?'
                    | b'^'
                    | b'_'
                    | b'`'
                    | b'{'
                    | b'|'
                    | b'}'
                    | b'~'
            )
    }) {
        return None;
    }
    for label in domain.split('.') {
        if label.is_empty()
            || label.len() > 63
            || label.starts_with('-')
            || label.ends_with('-')
            || !label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return None;
        }
    }
    Some(format!("{}@{}", local, domain.to_ascii_lowercase()))
}

pub(super) fn normalize_url(raw: &str) -> Option<String> {
    if raw.is_empty() || raw.len() > 2048 || raw.bytes().any(|b| b <= 0x20 || b == 0x7f) {
        return None;
    }
    let parsed = url::Url::parse(raw).ok()?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return None;
    }
    Some(parsed.to_string())
}
