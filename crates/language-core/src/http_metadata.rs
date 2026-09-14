use std::fmt;

const MAX_MEDIA_TYPE_BYTES: usize = 128;
const MAX_FILE_NAME_BYTES: usize = 255;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaType(String);

impl MediaType {
    pub fn new(raw: impl Into<String>) -> Option<Self> {
        let raw = raw.into();
        if raw.is_empty()
            || raw.len() > MAX_MEDIA_TYPE_BYTES
            || raw.bytes().any(|b| b < 0x20 || b >= 0x7f)
        {
            return None;
        }
        let essence = raw.split(';').next()?.trim();
        let (kind, subtype) = essence.split_once('/')?;
        if !valid_media_token(kind) || !valid_media_token(subtype) {
            return None;
        }
        Some(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn valid_media_token(raw: &str) -> bool {
    !raw.is_empty()
        && raw.bytes().all(|b| {
            b.is_ascii_alphanumeric()
                || matches!(
                    b,
                    b'!' | b'#' | b'$' | b'&' | b'^' | b'_' | b'.' | b'+' | b'-'
                )
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileName(String);

impl FileName {
    pub fn new(raw: impl Into<String>) -> Option<Self> {
        let raw = raw.into();
        if raw.is_empty()
            || raw.len() > MAX_FILE_NAME_BYTES
            || raw == "."
            || raw == ".."
            || raw.starts_with('.')
            || raw
                .bytes()
                .any(|b| !(b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b' ')))
        {
            return None;
        }
        Some(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentDisposition {
    Inline,
    Attachment(FileName),
}

impl ContentDisposition {
    pub fn attachment(file_name: FileName) -> Self {
        Self::Attachment(file_name)
    }
}

impl fmt::Display for ContentDisposition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inline => f.write_str("inline"),
            Self::Attachment(file_name) => {
                write!(f, "attachment; filename=\"{}\"", file_name.as_str())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ContentDisposition, FileName, MediaType};

    #[test]
    fn file_name_is_single_segment_and_header_safe() {
        let name = FileName::new("report-2026.pdf").unwrap();
        assert_eq!(
            ContentDisposition::attachment(name).to_string(),
            "attachment; filename=\"report-2026.pdf\""
        );
        assert!(FileName::new("../secret.txt").is_none());
        assert!(FileName::new("dir/file.txt").is_none());
        assert!(FileName::new("line\r\nbreak.txt").is_none());
        assert!(FileName::new(".hidden").is_none());
    }

    #[test]
    fn media_type_rejects_header_injection() {
        assert!(MediaType::new("application/json; charset=utf-8").is_some());
        assert!(MediaType::new("image/png").is_some());
        assert!(MediaType::new("text/plain\r\nX-Evil: 1").is_none());
        assert!(MediaType::new("not-a-media-type").is_none());
    }
}
