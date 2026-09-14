#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
}
impl HttpMethod {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "GET" => Some(Self::Get),
            "POST" => Some(Self::Post),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Html(String);
impl Html {
    pub fn trusted_compiler_output(value: String) -> Self {
        Self(value)
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalUrl(String);

impl LocalUrl {
    pub fn parse(value: String) -> Option<Self> {
        if !value.starts_with('/')
            || value.starts_with("//")
            || value.contains('\\')
            || value.bytes().any(|byte| matches!(byte, b'\r' | b'\n' | 0))
        {
            return None;
        }
        Some(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedirectStatus {
    SeeOther,
    MovedPermanently,
}
impl RedirectStatus {
    pub fn code(self) -> u16 {
        match self {
            Self::SeeOther => 303,
            Self::MovedPermanently => 301,
        }
    }
    pub fn reason(self) -> &'static str {
        match self {
            Self::SeeOther => "See Other",
            Self::MovedPermanently => "Moved Permanently",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashKind {
    Success,
    Info,
    Warning,
    Error,
}
impl FlashKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlashMessage {
    pub kind: FlashKind,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redirect {
    location: LocalUrl,
    status: RedirectStatus,
    flash: Option<FlashMessage>,
}
impl Redirect {
    pub fn new(location: LocalUrl) -> Self {
        Self {
            location,
            status: RedirectStatus::SeeOther,
            flash: None,
        }
    }
    pub fn permanent(location: LocalUrl) -> Self {
        Self {
            location,
            status: RedirectStatus::MovedPermanently,
            flash: None,
        }
    }
    pub fn with_flash(mut self, flash: FlashMessage) -> Self {
        self.flash = Some(flash);
        self
    }
    pub fn flash(&self) -> Option<&FlashMessage> {
        self.flash.as_ref()
    }
    pub fn location(&self) -> &str {
        self.location.as_str()
    }
    pub fn status(&self) -> RedirectStatus {
        self.status
    }
}

#[cfg(test)]
mod tests {
    use super::LocalUrl;

    #[test]
    fn local_url_rejects_external_and_protocol_relative_targets() {
        assert!(LocalUrl::parse("/account?tab=security".into()).is_some());
        assert!(LocalUrl::parse("//evil.example".into()).is_none());
        assert!(LocalUrl::parse("https://evil.example".into()).is_none());
        assert!(LocalUrl::parse("/x\r\nLocation: https://evil.example".into()).is_none());
    }
}
