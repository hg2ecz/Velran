use crate::diagnostics::CompileError;

pub(super) fn validate_publish_transition(
    upload_type: &str,
    binding_name: &str,
    publish: bool,
) -> Result<(), CompileError> {
    if upload_type == "Image" && !publish {
        return Err(CompileError::security(
            "SEC-FILE-001",
            format!("Image upload `{binding_name}` requires an explicit `publish` transition"),
            Some(
                "use `upload name<Image> to \"media\" publish`; unverified Upload values remain private and are never served by the media endpoint"
                    .into(),
            ),
        ));
    }
    if upload_type == "Upload" && publish {
        return Err(CompileError::security(
            "SEC-FILE-002",
            format!("raw Upload `{binding_name}` cannot be published"),
            Some("publish only an inspected Image; raw uploads stay private".into()),
        ));
    }
    Ok(())
}
