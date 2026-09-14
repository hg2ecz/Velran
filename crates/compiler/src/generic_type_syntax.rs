pub(crate) fn unwrap_generic<'a>(raw: &'a str, wrapper: &str) -> Option<&'a str> {
    let prefix = format!("{wrapper}<");
    if !raw.starts_with(&prefix) || !raw.ends_with('>') {
        return None;
    }
    let inner = &raw[prefix.len()..raw.len() - 1];
    if inner.is_empty() {
        return None;
    }
    let mut depth = 0i32;
    for ch in inner.chars() {
        match ch {
            '<' => depth += 1,
            '>' => {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
            }
            _ => {}
        }
    }
    (depth == 0).then_some(inner)
}
