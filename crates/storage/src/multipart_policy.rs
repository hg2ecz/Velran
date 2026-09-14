use crate::UploadError;
use multer::{Constraints, SizeLimit};

pub(crate) fn constraints(
    max_body_bytes: u64,
    max_file_bytes: u64,
    file_field_name: &str,
    text_field_names: &[String],
    max_text_field_bytes: u64,
) -> Result<Constraints, UploadError> {
    if file_field_name.is_empty() || text_field_names.len() > 1024 || max_text_field_bytes == 0 {
        return Err(UploadError::InvalidMultipart);
    }
    let mut allowed_fields = Vec::with_capacity(text_field_names.len() + 2);
    allowed_fields.push("_csrf".to_string());
    allowed_fields.push(file_field_name.to_string());
    allowed_fields.extend(text_field_names.iter().cloned());

    let mut limits = SizeLimit::new()
        .whole_stream(max_body_bytes)
        .per_field(max_file_bytes)
        .for_field("_csrf", 4096)
        .for_field(file_field_name, max_file_bytes);
    for name in text_field_names {
        limits = limits.for_field(name.as_str(), max_text_field_bytes);
    }
    Ok(Constraints::new()
        .allowed_fields(allowed_fields)
        .size_limit(limits))
}
