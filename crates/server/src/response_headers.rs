const MAX_HEADER_VALUE_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HeaderName {
    Location,
    CacheControl,
    Etag,
    ContentDisposition,
    Allow,
    Vary,
    ContentEncoding,
    XStaticAsset,
    SetCookie,
    AccessControlAllowOrigin,
    AccessControlAllowMethods,
    AccessControlAllowHeaders,
    AccessControlMaxAge,
    AccessControlAllowCredentials,
    RetryAfter,
    XRequestId,
}

impl HeaderName {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Location => "Location",
            Self::CacheControl => "Cache-Control",
            Self::Etag => "ETag",
            Self::ContentDisposition => "Content-Disposition",
            Self::Allow => "Allow",
            Self::Vary => "Vary",
            Self::ContentEncoding => "Content-Encoding",
            Self::XStaticAsset => "X-Static-Asset",
            Self::SetCookie => "Set-Cookie",
            Self::AccessControlAllowOrigin => "Access-Control-Allow-Origin",
            Self::AccessControlAllowMethods => "Access-Control-Allow-Methods",
            Self::AccessControlAllowHeaders => "Access-Control-Allow-Headers",
            Self::AccessControlMaxAge => "Access-Control-Max-Age",
            Self::AccessControlAllowCredentials => "Access-Control-Allow-Credentials",
            Self::RetryAfter => "Retry-After",
            Self::XRequestId => "X-Request-Id",
        }
    }

    pub(super) fn parse(raw: &str) -> Option<Self> {
        match raw.to_ascii_lowercase().as_str() {
            "location" => Some(Self::Location),
            "cache-control" => Some(Self::CacheControl),
            "etag" => Some(Self::Etag),
            "content-disposition" => Some(Self::ContentDisposition),
            "allow" => Some(Self::Allow),
            "vary" => Some(Self::Vary),
            "content-encoding" => Some(Self::ContentEncoding),
            "x-static-asset" => Some(Self::XStaticAsset),
            "set-cookie" => Some(Self::SetCookie),
            "access-control-allow-origin" => Some(Self::AccessControlAllowOrigin),
            "access-control-allow-methods" => Some(Self::AccessControlAllowMethods),
            "access-control-allow-headers" => Some(Self::AccessControlAllowHeaders),
            "access-control-max-age" => Some(Self::AccessControlMaxAge),
            "access-control-allow-credentials" => Some(Self::AccessControlAllowCredentials),
            "retry-after" => Some(Self::RetryAfter),
            "x-request-id" => Some(Self::XRequestId),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HeaderValue(String);

impl HeaderValue {
    pub(super) fn new(raw: impl Into<String>) -> Option<Self> {
        let raw = raw.into();
        if raw.is_empty()
            || raw.len() > MAX_HEADER_VALUE_BYTES
            || raw
                .bytes()
                .any(|b| b == b'\r' || b == b'\n' || (b < 0x20 && b != b'\t') || b == 0x7f)
        {
            return None;
        }
        Some(Self(raw))
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ResponseHeader {
    name: HeaderName,
    value: HeaderValue,
}

impl ResponseHeader {
    pub(super) fn new(name: HeaderName, value: impl Into<String>) -> Option<Self> {
        Some(Self {
            name,
            value: HeaderValue::new(value)?,
        })
    }

    pub(super) fn name(&self) -> HeaderName {
        self.name
    }

    pub(super) fn value(&self) -> &str {
        self.value.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::HeaderValue;

    #[test]
    fn header_value_rejects_response_splitting() {
        assert!(HeaderValue::new("safe").is_some());
        assert!(HeaderValue::new("safe\tvalue").is_some());
        assert!(HeaderValue::new("ok\r\nSet-Cookie: injected=1").is_none());
        assert!(HeaderValue::new("bad\nvalue").is_none());
    }
}
