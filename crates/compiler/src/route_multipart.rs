use crate::diagnostics::CompileError;
use language_core::{FormField, Program, UploadField};

pub(super) fn parse(
    tokens: &[String],
    cursor: &mut usize,
    route_name: &str,
    namespace: &str,
    program: &Program,
) -> Result<(Vec<FormField>, UploadField, String), CompileError> {
    *cursor += 1;
    let schema_token = tokens
        .get(*cursor)
        .ok_or_else(|| CompileError::Syntax("multipart requires a Rust struct schema".into()))?;
    if schema_token.contains('<') {
        return Err(CompileError::Syntax(
            "multipart requires a named Rust struct schema".into(),
        ));
    }
    let (fields, mut upload, schema_name) =
        crate::route_typed_schema::multipart_fields(route_name, schema_token, namespace, program)?;
    *cursor += 1;
    if tokens.get(*cursor).map(String::as_str) != Some("to") {
        return Err(CompileError::Syntax(
            "multipart requires `to <relative-path>`".into(),
        ));
    }
    let destination = tokens
        .get(*cursor + 1)
        .ok_or_else(|| CompileError::Syntax("multipart destination expected".into()))?
        .clone();
    crate::route_upload::validate_destination(&destination)?;
    if upload.image
        && !destination
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_'))
    {
        return Err(CompileError::Syntax(
            "Image multipart destination must be URL-safe (letters, digits, /, -, _)".into(),
        ));
    }
    let publish = tokens.get(*cursor + 2).map(String::as_str) == Some("publish");
    crate::upload_security::validate_publish_transition(
        if upload.image { "Image" } else { "Upload" },
        &upload.name,
        publish,
    )?;
    upload.destination = destination;
    upload.publish = publish;
    *cursor += if publish { 3 } else { 2 };
    Ok((fields, upload, schema_name))
}
