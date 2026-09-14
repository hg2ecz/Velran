use crate::StaticAssets;
use crate::http_io::{HttpRequest, Response};
use crate::response_headers::HeaderName;
use crate::server_errors::StaticPrefixError;
use language_core::{Route, RouteSegment};
use sha2::{Digest, Sha256};
use std::path::Path;
use storage::{AppFs, inspect_image};

pub(super) fn validate_static_prefix(raw: &str) -> Result<String, StaticPrefixError> {
    if !raw.starts_with('/')
        || !raw.ends_with('/')
        || raw.len() < 3
        || raw.contains("//")
        || raw.contains('\\')
        || raw.contains('?')
        || raw.contains('#')
    {
        return Err(StaticPrefixError::Shape);
    }
    if raw.split('/').any(|p| p == "." || p == "..") {
        return Err(StaticPrefixError::DotSegment);
    }
    let inner = raw.trim_matches('/');
    if inner.contains('/')
        || !inner
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(StaticPrefixError::UnsafeSegment);
    }
    Ok(raw.to_string())
}

pub(super) fn route_conflicts_static(route: &Route, prefix: &str) -> bool {
    let first = prefix.trim_matches('/').split('/').next().unwrap_or("");
    match route.segments.first() {
        Some(RouteSegment::Param { .. }) => true,
        Some(RouteSegment::Static(value)) => value == first,
        None => false,
    }
}

pub(super) async fn serve_media_image(
    program: &language_core::Program,
    appfs: Option<&AppFs>,
    request: &HttpRequest,
    path: &str,
    max_image_pixels: u64,
) -> Response {
    let method = request.method.as_str();
    if !matches!(method, "GET" | "HEAD") {
        return Response::text(405, "Method Not Allowed", b"method not allowed\n");
    }
    let Some(relative) = path.strip_prefix("/__velran/media/") else {
        return Response::text(404, "Not Found", b"not found\n");
    };
    if relative.is_empty() || has_hidden_path_segment(relative) {
        return Response::text(404, "Not Found", b"not found\n");
    }
    let allowed = program
        .routes
        .iter()
        .filter_map(|r| r.upload.as_ref())
        .filter(|u| u.image && u.publish)
        .any(|u| {
            let prefix = format!("{}/", u.destination.trim_end_matches('/'));
            relative.starts_with(&prefix) && !relative[prefix.len()..].contains('/')
        });
    if !allowed {
        return Response::text(404, "Not Found", b"not found\n");
    }
    let Some(fs) = appfs else {
        return Response::text(404, "Not Found", b"not found\n");
    };
    let bytes = match fs.read(relative).await {
        Ok(v) => v,
        Err(_) => return Response::text(404, "Not Found", b"not found\n"),
    };
    let image = match inspect_image(&bytes, max_image_pixels) {
        Ok(v) => v,
        Err(_) => return Response::text(404, "Not Found", b"not found\n"),
    };
    let digest = Sha256::digest(&bytes);
    let mut etag = String::from("\"");
    for b in &digest[..16] {
        use std::fmt::Write;
        let _ = write!(etag, "{b:02x}");
    }
    etag.push('"');
    if request.header("if-none-match") == Some(etag.as_str()) {
        let mut r = Response::new(304, "Not Modified", image.content_type, b"");
        r.push_header(
            HeaderName::CacheControl,
            "public, max-age=31536000, immutable",
        );
        r.push_header(HeaderName::Etag, etag);
        return r;
    }
    let mut r = Response::new(200, "OK", image.content_type, &bytes);
    r.push_header(
        HeaderName::CacheControl,
        "public, max-age=31536000, immutable",
    );
    r.push_header(HeaderName::Etag, etag);
    r.push_header(
        HeaderName::ContentDisposition,
        language_core::ContentDisposition::Inline.to_string(),
    );
    if method == "HEAD" {
        r.content_length_override = Some(bytes.len());
        r.body.clear();
        r.suppress_body = true;
    }
    r
}

pub(super) async fn serve_static_asset(
    assets: &StaticAssets,
    request: &HttpRequest,
    path: &str,
) -> Response {
    if request.method != "GET" && request.method != "HEAD" {
        let mut r = Response::text(
            405,
            "Method Not Allowed",
            b"static assets support GET and HEAD only\n",
        );
        r.push_header(HeaderName::Allow, "GET, HEAD");
        return r;
    }
    let Some(relative) = path.strip_prefix(&assets.url_prefix) else {
        return Response::text(404, "Not Found", b"not found\n");
    };
    if !valid_static_relative(relative) {
        return Response::text(400, "Bad Request", b"invalid static asset path\n");
    }

    let accept_encoding = request.header("accept-encoding").unwrap_or("");
    let mut encoding: Option<&'static str> = None;
    let mut bytes: Option<Vec<u8>> = None;
    if assets.precompressed && encoding_accepted(accept_encoding, "br") {
        let candidate = format!("{relative}.br");
        if let Ok(v) = assets.fs.read(&candidate).await {
            encoding = Some("br");
            bytes = Some(v);
        }
    }
    if bytes.is_none() && assets.precompressed && encoding_accepted(accept_encoding, "gzip") {
        let candidate = format!("{relative}.gz");
        if let Ok(v) = assets.fs.read(&candidate).await {
            encoding = Some("gzip");
            bytes = Some(v);
        }
    }
    if bytes.is_none() {
        bytes = assets.fs.read(relative).await.ok();
    }
    let Some(bytes) = bytes else {
        return Response::text(404, "Not Found", b"not found\n");
    };
    let etag = static_etag(&bytes);
    if if_none_match(request.header("if-none-match"), &etag) {
        let mut r = Response::new(304, "Not Modified", static_content_type(relative), b"");
        r.push_header(HeaderName::Etag, etag);
        r.push_header(
            HeaderName::CacheControl,
            static_cache_control(assets, relative),
        );
        if assets.precompressed {
            r.push_header(HeaderName::Vary, "Accept-Encoding");
        }
        if let Some(enc) = encoding {
            r.push_header(HeaderName::ContentEncoding, enc);
        }
        return r;
    }
    let original_len = bytes.len();
    let mut r = Response::new(200, "OK", static_content_type(relative), &bytes);
    r.push_header(HeaderName::Etag, etag);
    r.push_header(
        HeaderName::CacheControl,
        static_cache_control(assets, relative),
    );
    r.push_header(HeaderName::XStaticAsset, "1");
    if assets.precompressed {
        r.push_header(HeaderName::Vary, "Accept-Encoding");
    }
    if let Some(enc) = encoding {
        r.push_header(HeaderName::ContentEncoding, enc);
    }
    if request.method == "HEAD" {
        r.content_length_override = Some(original_len);
        r.suppress_body = true;
    }
    r
}

fn has_hidden_path_segment(relative: &str) -> bool {
    relative.split('/').any(|part| part.starts_with('.'))
}

pub(super) fn valid_static_relative(relative: &str) -> bool {
    if relative.is_empty()
        || relative.len() > 1024
        || relative.starts_with('/')
        || relative.ends_with('/')
    {
        return false;
    }
    if relative.contains('%')
        || relative.contains('\\')
        || relative.contains('?')
        || relative.contains('#')
        || relative.contains('\0')
    {
        return false;
    }
    relative.split('/').all(|part| {
        !part.is_empty()
            && part != "."
            && part != ".."
            && !part.starts_with('.')
            && part.len() <= 255
            && part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'@'))
    })
}

pub(super) fn static_content_type(path: &str) -> &'static str {
    let ext = Path::new(path)
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" | "map" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "txt" => "text/plain; charset=utf-8",
        "xml" => "application/xml; charset=utf-8",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

pub(super) fn static_cache_control(assets: &StaticAssets, path: &str) -> String {
    if fingerprinted_asset(path) {
        format!(
            "public, max-age={}, immutable",
            assets.immutable_max_age_secs
        )
    } else {
        format!(
            "public, max-age={}, must-revalidate",
            assets.regular_max_age_secs
        )
    }
}

pub(super) fn fingerprinted_asset(path: &str) -> bool {
    let name = Path::new(path)
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("");
    name.split(|c| c == '.' || c == '-' || c == '_')
        .any(|part| part.len() >= 8 && part.bytes().all(|b| b.is_ascii_hexdigit()))
}

pub(super) fn static_etag(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("\"velran-{:016x}-{:x}\"", h, bytes.len())
}

pub(super) fn if_none_match(value: Option<&str>, etag: &str) -> bool {
    let Some(value) = value else {
        return false;
    };
    value.split(',').any(|v| {
        let v = v.trim();
        v == "*" || v == etag || v.strip_prefix("W/").map(|x| x == etag).unwrap_or(false)
    })
}

pub(super) fn encoding_accepted(header: &str, wanted: &str) -> bool {
    fn quality(raw: &str) -> Option<(&str, f32)> {
        let mut parts = raw.trim().split(';');
        let token = parts.next()?.trim();
        let mut q = 1.0f32;
        for part in parts {
            let part = part.trim();
            if let Some(value) = part.strip_prefix("q=") {
                q = value.parse::<f32>().ok()?;
            }
        }
        Some((token, q.clamp(0.0, 1.0)))
    }
    let mut wildcard = None;
    for raw in header.split(',') {
        let Some((token, q)) = quality(raw) else {
            continue;
        };
        if token.eq_ignore_ascii_case(wanted) {
            return q > 0.0;
        }
        if token == "*" {
            wildcard = Some(q);
        }
    }
    wildcard.unwrap_or(0.0) > 0.0
}

#[cfg(test)]
mod hidden_path_tests {
    use super::*;

    #[test]
    fn static_paths_reject_hidden_files_and_directories() {
        assert!(!valid_static_relative(".env"));
        assert!(!valid_static_relative("config/.secret.toml"));
        assert!(!valid_static_relative("assets/.cache/app.js"));
        assert!(valid_static_relative("assets/app.min.js"));
        assert!(valid_static_relative("config/server.toml"));
    }

    #[test]
    fn hidden_segment_helper_covers_nested_paths() {
        assert!(has_hidden_path_segment(".git/config"));
        assert!(has_hidden_path_segment("public/.private/key"));
        assert!(!has_hidden_path_segment("public/private/key"));
    }
}
