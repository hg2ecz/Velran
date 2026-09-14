#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HtmlCapabilities {
    stylesheet: bool,
    image: bool,
    media: bool,
    form: bool,
}

impl HtmlCapabilities {
    fn inspect(body: &[u8]) -> Self {
        let lower = String::from_utf8_lossy(body).to_ascii_lowercase();
        Self {
            stylesheet: lower.contains("rel=\"stylesheet\"") || lower.contains("rel='stylesheet'"),
            image: lower.contains("<img"),
            media: lower.contains("<audio")
                || lower.contains("<video")
                || lower.contains("<source"),
            form: lower.contains("<form"),
        }
    }
}

pub(super) fn content_security_policy(content_type: &str, body: &[u8]) -> String {
    if !content_type
        .split(';')
        .next()
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("text/html"))
    {
        return "default-src 'none'; base-uri 'none'; frame-ancestors 'none'; object-src 'none'"
            .into();
    }

    let capabilities = HtmlCapabilities::inspect(body);
    let style_src = if capabilities.stylesheet {
        "'self'"
    } else {
        "'none'"
    };
    let image_src = if capabilities.image {
        "'self'"
    } else {
        "'none'"
    };
    let media_src = if capabilities.media {
        "'self'"
    } else {
        "'none'"
    };
    let font_src = if capabilities.stylesheet {
        "'self'"
    } else {
        "'none'"
    };
    let form_action = if capabilities.form {
        "'self'"
    } else {
        "'none'"
    };

    format!(
        "default-src 'none'; script-src 'none'; style-src {style_src}; img-src {image_src}; font-src {font_src}; media-src {media_src}; connect-src 'none'; frame-src 'none'; worker-src 'none'; object-src 'none'; base-uri 'none'; form-action {form_action}; frame-ancestors 'none'"
    )
}

#[cfg(test)]
mod tests {
    use super::content_security_policy;

    #[test]
    fn html_without_resources_gets_minimal_policy() {
        let policy = content_security_policy("text/html; charset=utf-8", b"<main>safe</main>");
        assert!(policy.contains("script-src 'none'"));
        assert!(policy.contains("style-src 'none'"));
        assert!(policy.contains("img-src 'none'"));
        assert!(policy.contains("connect-src 'none'"));
        assert!(policy.contains("form-action 'none'"));
    }

    #[test]
    fn html_enables_only_observed_same_origin_capabilities() {
        let policy = content_security_policy(
            "text/html; charset=utf-8",
            br#"<link rel="stylesheet" href="/assets/app.css"><form action="/save"><img src="/x.png"><video src="/x.mp4"></video></form>"#,
        );
        assert!(policy.contains("style-src 'self'"));
        assert!(policy.contains("font-src 'self'"));
        assert!(policy.contains("img-src 'self'"));
        assert!(policy.contains("media-src 'self'"));
        assert!(policy.contains("form-action 'self'"));
        assert!(policy.contains("connect-src 'none'"));
    }

    #[test]
    fn non_html_documents_are_locked_down() {
        let policy = content_security_policy("application/json", br#"{\"ok\":true}"#);
        assert_eq!(
            policy,
            "default-src 'none'; base-uri 'none'; frame-ancestors 'none'; object-src 'none'"
        );
    }
}
