use crate::error::IntegrationError;
use std::collections::HashMap;
use tokio::io::{AsyncRead, AsyncReadExt};

const MAX_HEADER_BYTES: usize = 32 * 1024;
const MAX_HEADER_COUNT: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusResponse {
    pub status: u16,
    pub transferred_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpsResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

pub(crate) async fn read_http_response<S: AsyncRead + Unpin>(
    stream: &mut S,
    max_body: usize,
) -> Result<HttpsResponse, IntegrationError> {
    let mut data = Vec::new();
    let mut buf = [0u8; 4096];
    let head_end = loop {
        let n = stream
            .read(&mut buf)
            .await
            .map_err(|_| IntegrationError::Protocol)?;
        if n == 0 {
            return Err(IntegrationError::Protocol);
        }
        data.extend_from_slice(&buf[..n]);
        if data.len() > MAX_HEADER_BYTES {
            return Err(IntegrationError::Protocol);
        }
        if let Some(pos) = find_double_crlf(&data) {
            break pos + 4;
        }
    };
    let head = std::str::from_utf8(&data[..head_end]).map_err(|_| IntegrationError::Protocol)?;
    let mut lines = head[..head.len() - 4].split("\r\n");
    let status_line = lines.next().ok_or(IntegrationError::Protocol)?;
    let mut parts = status_line.split_whitespace();
    if parts.next() != Some("HTTP/1.1") {
        return Err(IntegrationError::Protocol);
    }
    let status = parts
        .next()
        .ok_or(IntegrationError::Protocol)?
        .parse::<u16>()
        .map_err(|_| IntegrationError::Protocol)?;
    let mut headers = HashMap::new();
    let mut content_length = None;
    for (count, line) in lines.enumerate() {
        if count >= MAX_HEADER_COUNT {
            return Err(IntegrationError::Protocol);
        }
        let (name, value) = line.split_once(':').ok_or(IntegrationError::Protocol)?;
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        validate_header(&name, value)?;
        if headers.contains_key(&name) {
            return Err(IntegrationError::Protocol);
        }
        if name == "transfer-encoding" {
            return Err(IntegrationError::Protocol);
        }
        if name == "content-encoding" && !value.eq_ignore_ascii_case("identity") {
            return Err(IntegrationError::Protocol);
        }
        if name == "content-length" {
            content_length = Some(
                value
                    .parse::<usize>()
                    .map_err(|_| IntegrationError::Protocol)?,
            );
        }
        headers.insert(name, value.to_string());
    }
    let expected = content_length.unwrap_or(0);
    if expected > max_body {
        return Err(IntegrationError::ResponseTooLarge);
    }
    let mut body = data[head_end..].to_vec();
    if body.len() > expected {
        return Err(IntegrationError::Protocol);
    }
    while body.len() < expected {
        let want = (expected - body.len()).min(buf.len());
        let n = stream
            .read(&mut buf[..want])
            .await
            .map_err(|_| IntegrationError::Protocol)?;
        if n == 0 {
            return Err(IntegrationError::Protocol);
        }
        body.extend_from_slice(&buf[..n]);
        if body.len() > max_body {
            return Err(IntegrationError::ResponseTooLarge);
        }
    }
    Ok(HttpsResponse {
        status,
        headers,
        body,
    })
}

fn validate_header(name: &str, value: &str) -> Result<(), IntegrationError> {
    if name.is_empty()
        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        || value.bytes().any(|b| matches!(b, b'\r' | b'\n' | 0))
    {
        return Err(IntegrationError::Protocol);
    }
    Ok(())
}

fn find_double_crlf(value: &[u8]) -> Option<usize> {
    value.windows(4).position(|w| w == b"\r\n\r\n")
}

#[cfg(test)]
mod tests {
    use super::read_http_response;
    use crate::IntegrationError;

    #[tokio::test]
    async fn rejects_compressed_upstream_response() {
        let mut response: &[u8] =
            b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nContent-Encoding: gzip\r\n\r\ntest";
        assert_eq!(
            read_http_response(&mut response, 1024).await,
            Err(IntegrationError::Protocol)
        );
    }

    #[tokio::test]
    async fn rejects_body_over_cap_before_reading_it() {
        let mut response: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello";
        assert_eq!(
            read_http_response(&mut response, 4).await,
            Err(IntegrationError::ResponseTooLarge)
        );
    }
}
