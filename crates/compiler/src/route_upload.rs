use crate::diagnostics::CompileError;
use crate::source_syntax::is_identifier;
use language_core::UploadField;

pub(super) fn parse_legacy(
    tokens: &[String],
    cursor: &mut usize,
) -> Result<UploadField, CompileError> {
    *cursor += 1;
    let binding = tokens
        .get(*cursor)
        .ok_or_else(|| CompileError::Syntax("upload binding expected".into()))?;
    let lt = binding.find('<').ok_or_else(|| {
        CompileError::Syntax("upload binding must use name<Upload> or name<Image>".into())
    })?;
    if !binding.ends_with('>') {
        return Err(CompileError::Syntax(
            "upload binding must use name<Upload> or name<Image>".into(),
        ));
    }
    let upload_ty = &binding[lt + 1..binding.len() - 1];
    if !matches!(upload_ty, "Upload" | "Image") {
        return Err(CompileError::Syntax(
            "upload binding must use name<Upload> or name<Image>".into(),
        ));
    }
    let name = &binding[..lt];
    if !is_identifier(name) {
        return Err(CompileError::Syntax("invalid upload binding name".into()));
    }
    if tokens.get(*cursor + 1).map(String::as_str) != Some("to") {
        return Err(CompileError::Syntax(
            "upload binding requires `to <relative-path>`".into(),
        ));
    }
    let destination = tokens
        .get(*cursor + 2)
        .ok_or_else(|| CompileError::Syntax("upload destination expected".into()))?
        .clone();
    validate_destination(&destination)?;
    if upload_ty == "Image"
        && !destination
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_'))
    {
        return Err(CompileError::Syntax(
            "Image upload destination must be URL-safe (letters, digits, /, -, _)".into(),
        ));
    }
    let publish = tokens.get(*cursor + 3).map(String::as_str) == Some("publish");
    crate::upload_security::validate_publish_transition(upload_ty, name, publish)?;
    *cursor += if publish { 4 } else { 3 };
    Ok(UploadField {
        name: name.into(),
        destination,
        image: upload_ty == "Image",
        publish,
    })
}

pub(super) fn validate_destination(value: &str) -> Result<(), CompileError> {
    if value.is_empty()
        || value.len() > 4096
        || value.starts_with('/')
        || value.contains('\\')
        || value.as_bytes().contains(&0)
        || value
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(CompileError::Syntax(
            "upload destination must be a safe relative AppFs path".into(),
        ));
    }
    Ok(())
}
