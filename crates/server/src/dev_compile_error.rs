use crate::Response;
use std::collections::BTreeMap;
use std::sync::{OnceLock, RwLock};

const MAX_DIAGNOSTIC_CHARS: usize = 64 * 1024;
static LAST_ERRORS: OnceLock<RwLock<BTreeMap<String, String>>> = OnceLock::new();

fn slot() -> &'static RwLock<BTreeMap<String, String>> {
    LAST_ERRORS.get_or_init(|| RwLock::new(BTreeMap::new()))
}

pub(super) fn set(domain: &str, text: &str) {
    let bounded: String = text.chars().take(MAX_DIAGNOSTIC_CHARS).collect();
    if let Ok(mut guard) = slot().write() {
        guard.insert(domain.to_owned(), bounded);
    }
}

pub(super) fn clear(domain: &str) {
    if let Ok(mut guard) = slot().write() {
        guard.remove(domain);
    }
}

pub(super) fn response(domain: &str, method: &str) -> Option<Response> {
    if !matches!(method, "GET" | "HEAD") {
        return None;
    }
    let text = slot().read().ok()?.get(domain).cloned()?;
    let html = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Velran compile error</title><style>body{{font-family:ui-monospace,SFMono-Regular,Consolas,monospace;max-width:1100px;margin:2rem auto;padding:0 1rem}}pre{{white-space:pre-wrap;background:#111;color:#eee;padding:1rem;overflow:auto}}h1{{font-family:system-ui,sans-serif}}</style></head><body><h1>Velran candidate build rejected</h1><p>The last valid generation is still serving. Fix the source and save again.</p><pre>{}</pre></body></html>",
        html_escape(&text)
    );
    let mut response = Response::new(
        500,
        "Internal Server Error",
        "text/html; charset=utf-8",
        html.as_bytes(),
    );
    if method == "HEAD" {
        response.content_length_override = Some(response.body.len());
        response.suppress_body = true;
    }
    Some(response)
}

fn html_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_page_escapes_untrusted_compiler_text_and_is_domain_scoped() {
        clear("a.test");
        clear("b.test");
        set("a.test", "error: <script>alert('x')</script>");
        assert!(response("b.test", "GET").is_none());
        let response = response("a.test", "GET").unwrap();
        let body = String::from_utf8(response.body).unwrap();
        assert!(!body.contains("<script>"));
        assert!(body.contains("&lt;script&gt;"));
        clear("a.test");
    }
}
