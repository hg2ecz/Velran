use crate::filesystem::{AppFs, FsError};

#[derive(Debug)]
pub struct UploadResult {
    pub(crate) staging_path: String,
    pub bytes_written: u64,
    pub csrf_token: String,
    pub original_filename: Option<String>,
    pub content_type: Option<String>,
    pub text_fields: Vec<(String, String)>,
}

#[derive(Debug)]
pub enum UploadError {
    InvalidContentType,
    InvalidMultipart,
    FieldCardinality,
    InvalidFilename,
    Fs(FsError),
}

impl std::fmt::Display for UploadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidContentType => f.write_str("invalid multipart content type"),
            Self::InvalidMultipart => f.write_str("invalid multipart form"),
            Self::FieldCardinality => {
                f.write_str("required multipart field is missing or duplicated")
            }
            Self::InvalidFilename => f.write_str("multipart filename is invalid"),
            Self::Fs(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for UploadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Fs(error) => Some(error),
            _ => None,
        }
    }
}

impl From<FsError> for UploadError {
    fn from(value: FsError) -> Self {
        Self::Fs(value)
    }
}

pub fn multipart_boundary(content_type: &str) -> Result<String, UploadError> {
    if !content_type
        .to_ascii_lowercase()
        .starts_with("multipart/form-data;")
    {
        return Err(UploadError::InvalidContentType);
    }
    multer::parse_boundary(content_type).map_err(|_| UploadError::InvalidContentType)
}

pub async fn store_single_multipart_file<R>(
    reader: R,
    boundary: &str,
    appfs: &AppFs,
    destination: &str,
    max_body_bytes: u64,
    expected_csrf: &str,
    file_field_name: &str,
    text_field_names: &[String],
    max_text_field_bytes: u64,
) -> Result<UploadResult, UploadError>
where
    R: tokio::io::AsyncRead + Unpin + Send,
{
    use multer::Multipart;

    if !appfs.allows_create() {
        return Err(FsError::Denied.into());
    }
    appfs.validate_upload_destination(destination)?;
    let constraints = crate::multipart_policy::constraints(
        max_body_bytes,
        appfs.limits().max_file_bytes,
        file_field_name,
        text_field_names,
        max_text_field_bytes,
    )?;
    let mut multipart =
        Multipart::with_reader_with_constraints(reader, boundary.to_owned(), constraints);
    let mut csrf: Option<String> = None;
    let mut csrf_verified = false;
    let mut upload: Option<(u64, Option<String>, Option<String>)> = None;
    let mut staged_path: Option<String> = None;
    let mut text_fields: Vec<(String, String)> = Vec::new();

    let parsed: Result<(), UploadError> = async {
        while let Some(mut field) = multipart
            .next_field()
            .await
            .map_err(|_| UploadError::InvalidMultipart)?
        {
            let name = field
                .name()
                .ok_or(UploadError::InvalidMultipart)?
                .to_string();
            match name.as_str() {
                "_csrf" => {
                    if csrf.is_some() || upload.is_some() {
                        return Err(UploadError::FieldCardinality);
                    }
                    let text = field
                        .text()
                        .await
                        .map_err(|_| UploadError::InvalidMultipart)?;
                    if text.is_empty()
                        || text.len() > 4096
                        || text.bytes().any(|b| b < 0x21 || b > 0x7e)
                        || text != expected_csrf
                    {
                        return Err(UploadError::InvalidMultipart);
                    }
                    csrf_verified = true;
                    csrf = Some(text);
                }
                field_name if field_name == file_field_name => {
                    if !csrf_verified || upload.is_some() {
                        return Err(UploadError::FieldCardinality);
                    }
                    let filename = field
                        .file_name()
                        .map(validate_upload_filename)
                        .transpose()?;
                    let content_type = field.content_type().map(|v| v.to_string());
                    let (staging, mut out) = appfs.create_staged(destination).await?;
                    staged_path = Some(staging);
                    while let Some(chunk) = field
                        .chunk()
                        .await
                        .map_err(|_| UploadError::InvalidMultipart)?
                    {
                        out.write_chunk(&chunk).await?;
                    }
                    let bytes = out.finish().await?;
                    upload = Some((bytes, filename, content_type));
                }
                field_name if text_field_names.iter().any(|v| v == field_name) => {
                    if !csrf_verified || text_fields.iter().any(|(name, _)| name == field_name) {
                        return Err(UploadError::FieldCardinality);
                    }
                    let text = field
                        .text()
                        .await
                        .map_err(|_| UploadError::InvalidMultipart)?;
                    if text.len() as u64 > max_text_field_bytes || text.as_bytes().contains(&0) {
                        return Err(UploadError::InvalidMultipart);
                    }
                    text_fields.push((field_name.to_string(), text));
                }
                _ => return Err(UploadError::InvalidMultipart),
            }
        }
        Ok(())
    }
    .await;

    if let Err(err) = parsed {
        if let Some(staging) = staged_path.as_deref() {
            appfs.cleanup_staged(staging);
        }
        return Err(err);
    }
    let csrf_token = csrf.ok_or(UploadError::FieldCardinality)?;
    let (bytes_written, original_filename, content_type) =
        upload.ok_or(UploadError::FieldCardinality)?;
    let staging = staged_path
        .as_deref()
        .ok_or(UploadError::FieldCardinality)?;
    Ok(UploadResult {
        staging_path: staging.to_string(),
        bytes_written,
        csrf_token,
        original_filename,
        content_type,
        text_fields,
    })
}

fn validate_upload_filename(raw: &str) -> Result<String, UploadError> {
    if raw.is_empty()
        || raw.len() > 255
        || raw.contains('/')
        || raw.contains('\\')
        || raw.contains('\0')
        || raw == "."
        || raw == ".."
    {
        return Err(UploadError::InvalidFilename);
    }
    if raw.bytes().any(|b| b < 0x20 || b == 0x7f) {
        return Err(UploadError::InvalidFilename);
    }
    Ok(raw.to_string())
}
