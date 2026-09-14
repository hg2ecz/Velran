use storage::{AppFs, inspect_image};

use crate::server_errors::UploadRuntimeError;

pub(super) fn encode_upload_descriptor(destination: &str, info: &storage::UploadResult) -> Vec<u8> {
    let mut out = vec![1u8];
    push_text(&mut out, destination);
    push_text(&mut out, info.original_filename.as_deref().unwrap_or(""));
    push_text(&mut out, info.content_type.as_deref().unwrap_or(""));
    out.extend_from_slice(&info.bytes_written.to_le_bytes());
    out
}
pub(super) async fn encode_image_descriptor(
    fs: &AppFs,
    destination: &str,
    info: &storage::UploadResult,
    max_image_pixels: u64,
) -> Result<Vec<u8>, UploadRuntimeError> {
    let bytes = fs.read_staged_upload(info).await?;
    let image = inspect_image(&bytes, max_image_pixels)?;
    let mut out = vec![1u8];
    push_text(&mut out, destination);
    push_text(&mut out, image.content_type);
    out.extend_from_slice(&image.width.to_le_bytes());
    out.extend_from_slice(&image.height.to_le_bytes());
    out.extend_from_slice(&info.bytes_written.to_le_bytes());
    Ok(out)
}
fn push_text(out: &mut Vec<u8>, value: &str) {
    let len = u32::try_from(value.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(value.as_bytes());
}

pub(super) fn media_type_is(value: &str, wanted: &str) -> bool {
    value
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .eq_ignore_ascii_case(wanted)
}
