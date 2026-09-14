use crate::response_headers::{HeaderName, ResponseHeader};
use language_core::{MediaType, ServerConfig};
use std::io;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::time::timeout;

pub(super) struct Response {
    pub(super) status: u16,
    pub(super) reason: &'static str,
    content_type: MediaType,
    pub(super) body: Vec<u8>,
    headers: Vec<ResponseHeader>,
    metadata_valid: bool,
    pub(super) content_length_override: Option<usize>,
    pub(super) suppress_body: bool,
}

impl Response {
    pub(super) fn new(status: u16, reason: &'static str, content_type: &str, body: &[u8]) -> Self {
        let parsed_content_type = MediaType::new(content_type);
        let metadata_valid = parsed_content_type.is_some();
        Self {
            status,
            reason,
            content_type: parsed_content_type
                .unwrap_or_else(|| MediaType::new("application/octet-stream").unwrap()),
            body: body.to_vec(),
            headers: Vec::new(),
            metadata_valid,
            content_length_override: None,
            suppress_body: false,
        }
    }

    pub(super) fn text(status: u16, reason: &'static str, body: &[u8]) -> Self {
        Self::new(status, reason, "text/plain; charset=utf-8", body)
    }

    pub(super) fn redirect(status: u16, reason: &'static str, location: &str) -> Self {
        let mut response = Self::new(status, reason, "text/plain; charset=utf-8", b"redirect\n");
        response.push_header(HeaderName::Location, location);
        response
    }

    pub(super) fn content_type(&self) -> &str {
        self.content_type.as_str()
    }

    pub(super) fn push_header(&mut self, name: HeaderName, value: impl Into<String>) {
        let Some(header) = ResponseHeader::new(name, value) else {
            self.metadata_valid = false;
            return;
        };
        self.headers.push(header);
    }

    pub(super) fn has_header(&self, name: HeaderName) -> bool {
        self.headers.iter().any(|header| header.name() == name)
    }

    pub(super) fn remove_header(&mut self, name: HeaderName) {
        self.headers.retain(|header| header.name() != name);
    }

    pub(super) fn stored_headers(&self) -> Vec<(String, String)> {
        self.headers
            .iter()
            .map(|header| {
                (
                    header.name().as_str().to_string(),
                    header.value().to_string(),
                )
            })
            .collect()
    }

    pub(super) fn restore_headers(&mut self, headers: Vec<(String, String)>) -> bool {
        for (name, value) in headers {
            let Some(name) = HeaderName::parse(&name) else {
                self.metadata_valid = false;
                return false;
            };
            self.push_header(name, value);
        }
        self.metadata_valid
    }

    pub(super) fn headers(&self) -> impl Iterator<Item = (&str, &str)> {
        self.headers
            .iter()
            .map(|header| (header.name().as_str(), header.value()))
    }

    pub(super) fn metadata_valid(&self) -> bool {
        self.metadata_valid
    }
}

#[derive(Debug)]
pub(super) struct HttpRequest {
    pub(super) method: String,
    pub(super) target: String,
    pub(super) headers: Vec<(String, String)>,
    pub(super) body: Vec<u8>,
}

impl HttpRequest {
    pub(super) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug)]
pub(super) enum HttpReadError {
    HeaderTooLarge,
    BodyTooLarge,
    BadRequest,
    Io,
}

pub(super) async fn read_request_head<S>(
    stream: &mut S,
    buffer: &mut Vec<u8>,
    config: &ServerConfig,
) -> Result<Option<ParsedHead>, HttpReadError>
where
    S: AsyncRead + Unpin,
{
    let header_end = loop {
        if let Some(end) = find_header_end(buffer) {
            break end;
        }
        if buffer.len() >= config.max_header_bytes {
            return Err(HttpReadError::HeaderTooLarge);
        }
        let mut chunk = [0u8; 4096];
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|_| HttpReadError::Io)?;
        if read == 0 {
            if buffer.is_empty() {
                return Ok(None);
            }
            return Err(HttpReadError::BadRequest);
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.len() > config.max_header_bytes && find_header_end(buffer).is_none() {
            return Err(HttpReadError::HeaderTooLarge);
        }
    };
    if header_end + 4 > config.max_header_bytes {
        return Err(HttpReadError::HeaderTooLarge);
    }
    let head = std::str::from_utf8(&buffer[..header_end]).map_err(|_| HttpReadError::BadRequest)?;
    let parsed = parse_request_head(head, config.max_header_count)?;
    buffer.drain(..header_end + 4);
    Ok(Some(parsed))
}

pub(super) async fn read_buffered_body<S>(
    stream: &mut S,
    buffer: &mut Vec<u8>,
    content_length: usize,
) -> Result<Vec<u8>, HttpReadError>
where
    S: AsyncRead + Unpin,
{
    while buffer.len() < content_length {
        let remaining = content_length - buffer.len();
        let mut chunk = vec![0u8; remaining.min(8192)];
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|_| HttpReadError::Io)?;
        if read == 0 {
            return Err(HttpReadError::BadRequest);
        }
        buffer.extend_from_slice(&chunk[..read]);
    }
    Ok(buffer.drain(..content_length).collect())
}

pub(super) async fn read_request<S>(
    stream: &mut S,
    buffer: &mut Vec<u8>,
    config: &ServerConfig,
) -> Result<Option<HttpRequest>, HttpReadError>
where
    S: AsyncRead + Unpin,
{
    let header_end = loop {
        if let Some(end) = find_header_end(buffer) {
            break end;
        }
        if buffer.len() >= config.max_header_bytes {
            return Err(HttpReadError::HeaderTooLarge);
        }
        let mut chunk = [0u8; 4096];
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|_| HttpReadError::Io)?;
        if read == 0 {
            if buffer.is_empty() {
                return Ok(None);
            }
            return Err(HttpReadError::BadRequest);
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.len() > config.max_header_bytes && find_header_end(buffer).is_none() {
            return Err(HttpReadError::HeaderTooLarge);
        }
    };

    if header_end + 4 > config.max_header_bytes {
        return Err(HttpReadError::HeaderTooLarge);
    }
    let head = std::str::from_utf8(&buffer[..header_end]).map_err(|_| HttpReadError::BadRequest)?;
    let parsed = parse_request_head(head, config.max_header_count)?;
    if parsed.content_length > config.max_body_bytes {
        return Err(HttpReadError::BodyTooLarge);
    }

    let total = header_end + 4 + parsed.content_length;
    while buffer.len() < total {
        let remaining = total - buffer.len();
        let mut chunk = vec![0u8; remaining.min(8192)];
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|_| HttpReadError::Io)?;
        if read == 0 {
            return Err(HttpReadError::BadRequest);
        }
        buffer.extend_from_slice(&chunk[..read]);
    }

    let body = buffer[header_end + 4..total].to_vec();
    buffer.drain(..total);

    Ok(Some(HttpRequest {
        method: parsed.method,
        target: parsed.target,
        headers: parsed.headers,
        body,
    }))
}

pub(super) struct ParsedHead {
    pub(super) method: String,
    pub(super) target: String,
    pub(super) headers: Vec<(String, String)>,
    pub(super) content_length: usize,
    pub(super) keep_alive: bool,
}

pub(super) fn parse_request_head(
    head: &str,
    max_header_count: usize,
) -> Result<ParsedHead, HttpReadError> {
    let mut lines = head.split("\r\n");
    let request_line = lines.next().ok_or(HttpReadError::BadRequest)?;
    let mut parts = request_line.split(' ');
    let method = parts.next().ok_or(HttpReadError::BadRequest)?;
    let target = parts.next().ok_or(HttpReadError::BadRequest)?;
    let version = parts.next().ok_or(HttpReadError::BadRequest)?;
    if parts.next().is_some() || method.is_empty() || target.is_empty() || version != "HTTP/1.1" {
        return Err(HttpReadError::BadRequest);
    }
    if !crate::request_target_security::valid_origin_form_target(target) {
        return Err(HttpReadError::BadRequest);
    }
    if !method.bytes().all(|b| b.is_ascii_uppercase()) {
        return Err(HttpReadError::BadRequest);
    }

    let mut headers = Vec::new();
    let mut content_length = None;
    let mut host_count = 0usize;
    let mut cookie_count = 0usize;
    let mut origin_count = 0usize;
    let mut forwarded_count = 0usize;
    let mut xff_count = 0usize;
    let mut xfp_count = 0usize;
    let mut referer_count = 0usize;
    let mut fetch_site_count = 0usize;
    let mut content_type_count = 0usize;
    let mut csrf_header_count = 0usize;
    let mut acr_method_count = 0usize;
    let mut acr_headers_count = 0usize;
    let mut accept_count = 0usize;
    let mut connection_close = false;

    let mut header_count = 0usize;
    for line in lines {
        header_count += 1;
        if header_count > max_header_count {
            return Err(HttpReadError::HeaderTooLarge);
        }
        if line.is_empty() || line.starts_with(' ') || line.starts_with('\t') {
            return Err(HttpReadError::BadRequest);
        }
        let (name, value) = line.split_once(':').ok_or(HttpReadError::BadRequest)?;
        if !valid_header_name(name) {
            return Err(HttpReadError::BadRequest);
        }
        let value = value.trim();
        if value.bytes().any(|b| (b < 0x20 && b != b'\t') || b >= 0x7f) {
            return Err(HttpReadError::BadRequest);
        }
        let lower = name.to_ascii_lowercase();
        match lower.as_str() {
            "host" => {
                host_count += 1;
                if host_count > 1 || value.is_empty() {
                    return Err(HttpReadError::BadRequest);
                }
            }
            "cookie" => {
                cookie_count += 1;
                if cookie_count > 1 {
                    return Err(HttpReadError::BadRequest);
                }
            }
            "origin" => {
                origin_count += 1;
                if origin_count > 1 {
                    return Err(HttpReadError::BadRequest);
                }
            }
            "referer" => {
                referer_count += 1;
                if referer_count > 1 {
                    return Err(HttpReadError::BadRequest);
                }
            }
            "forwarded" => {
                forwarded_count += 1;
                if forwarded_count > 1 {
                    return Err(HttpReadError::BadRequest);
                }
            }
            "x-forwarded-for" => {
                xff_count += 1;
                if xff_count > 1 {
                    return Err(HttpReadError::BadRequest);
                }
            }
            "x-forwarded-proto" => {
                xfp_count += 1;
                if xfp_count > 1 || value.is_empty() {
                    return Err(HttpReadError::BadRequest);
                }
            }
            "sec-fetch-site" => {
                fetch_site_count += 1;
                if fetch_site_count > 1 {
                    return Err(HttpReadError::BadRequest);
                }
            }
            "content-type" => {
                content_type_count += 1;
                if content_type_count > 1 {
                    return Err(HttpReadError::BadRequest);
                }
            }
            "content-encoding" => {
                if !value.eq_ignore_ascii_case("identity") {
                    return Err(HttpReadError::BadRequest);
                }
            }
            "x-csrf-token" => {
                csrf_header_count += 1;
                if csrf_header_count > 1 || value.is_empty() {
                    return Err(HttpReadError::BadRequest);
                }
            }
            "access-control-request-method" => {
                acr_method_count += 1;
                if acr_method_count > 1 || value.is_empty() {
                    return Err(HttpReadError::BadRequest);
                }
            }
            "access-control-request-headers" => {
                acr_headers_count += 1;
                if acr_headers_count > 1 {
                    return Err(HttpReadError::BadRequest);
                }
            }
            "accept" => {
                accept_count += 1;
                if accept_count > 1 {
                    return Err(HttpReadError::BadRequest);
                }
            }
            "content-length" => {
                if content_length.is_some()
                    || value.is_empty()
                    || !value.bytes().all(|b| b.is_ascii_digit())
                {
                    return Err(HttpReadError::BadRequest);
                }
                content_length = Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| HttpReadError::BadRequest)?,
                );
            }
            "transfer-encoding" | "trailer" | "upgrade" | "proxy-connection" | "expect" => {
                return Err(HttpReadError::BadRequest);
            }
            "connection" => {
                for token in value.split(',').map(str::trim) {
                    if token.eq_ignore_ascii_case("close") {
                        connection_close = true;
                    } else if token.eq_ignore_ascii_case("keep-alive") {
                    } else {
                        return Err(HttpReadError::BadRequest);
                    }
                }
            }
            _ => {}
        }
        headers.push((lower, value.to_string()));
    }

    if host_count != 1 {
        return Err(HttpReadError::BadRequest);
    }
    let content_length = content_length.unwrap_or(0);
    if method != "POST" && content_length != 0 {
        return Err(HttpReadError::BadRequest);
    }
    if method == "POST" && content_length == 0 {
        return Err(HttpReadError::BadRequest);
    }
    Ok(ParsedHead {
        method: method.to_string(),
        target: target.to_string(),
        headers,
        content_length,
        keep_alive: !connection_close,
    })
}

fn valid_header_name(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(|b| matches!(b,
        b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~' |
        b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z'))
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

pub(super) async fn write_response_with_timeout<S>(
    stream: &mut S,
    config: &ServerConfig,
    response: Response,
    keep_alive: bool,
    is_tls: bool,
) -> io::Result<()>
where
    S: AsyncWrite + Unpin,
{
    if !response.metadata_valid() || response.headers().count() > 64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "response contains invalid typed HTTP metadata",
        ));
    }
    let deadline = Duration::from_millis(config.write_timeout_ms);
    let mut head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: {}\r\n",
        response.status,
        response.reason,
        response.content_type(),
        response
            .content_length_override
            .unwrap_or(response.body.len()),
        if keep_alive { "keep-alive" } else { "close" },
    );
    head.push_str("X-Content-Type-Options: nosniff\r\n");
    if is_tls {
        head.push_str("Strict-Transport-Security: max-age=31536000\r\n");
    }
    head.push_str("Referrer-Policy: no-referrer\r\n");
    let csp =
        crate::security_headers::content_security_policy(response.content_type(), &response.body);
    head.push_str("Content-Security-Policy: ");
    head.push_str(&csp);
    head.push_str("\r\n");
    head.push_str("Cross-Origin-Opener-Policy: same-origin\r\n");
    let cors_response = response.has_header(HeaderName::AccessControlAllowOrigin);
    head.push_str(if cors_response {
        "Cross-Origin-Resource-Policy: cross-origin\r\n"
    } else {
        "Cross-Origin-Resource-Policy: same-origin\r\n"
    });
    head.push_str("X-Frame-Options: DENY\r\n");
    if !response.has_header(HeaderName::CacheControl) {
        head.push_str("Cache-Control: no-store\r\n");
    }
    head.push_str("Permissions-Policy: camera=(), microphone=(), geolocation=()\r\n");
    for (name, value) in response.headers() {
        head.push_str(name);
        head.push_str(": ");
        head.push_str(value);
        head.push_str("\r\n");
    }
    head.push_str("\r\n");

    timeout(deadline, async {
        stream.write_all(head.as_bytes()).await?;
        if !response.suppress_body {
            stream.write_all(&response.body).await?;
        }
        stream.flush().await
    })
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "response write timeout"))??;
    Ok(())
}

#[cfg(test)]
mod response_metadata_tests {
    use super::Response;
    use crate::response_headers::HeaderName;

    #[test]
    fn invalid_dynamic_header_marks_response_fail_closed() {
        let mut response = Response::text(200, "OK", b"ok");
        response.push_header(HeaderName::Location, "/safe\r\nSet-Cookie: injected=1");
        assert!(!response.metadata_valid());
    }

    #[test]
    fn invalid_media_type_marks_response_fail_closed() {
        let response = Response::new(200, "OK", "text/plain\r\nX-Evil: 1", b"ok");
        assert!(!response.metadata_valid());
    }
}

#[cfg(test)]
mod adversarial_tests;
