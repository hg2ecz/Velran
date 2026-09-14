use crate::declarations;
use crate::diagnostics::CompileError;
use crate::module_namespace::{qualify, resolve};
use crate::source_syntax::{is_identifier, read_ident};
use language_core::{Program, SecurityEvent};

pub(super) fn parse_security_events(
    source: &str,
    namespace: &str,
    program: &mut Program,
) -> Result<(), CompileError> {
    let mut offset = 0usize;
    while let Some(relative) = source[offset..].find("security event ") {
        let keyword = offset + relative;
        if !declarations::is_top_level_declaration_at(source, keyword) {
            offset = keyword + "security event ".len();
            continue;
        }
        let start = keyword + "security event ".len();
        let name = read_ident(source, start)
            .ok_or_else(|| CompileError::Syntax("security event name expected".into()))?;
        validate_name(&name)?;
        let after_name = source[start + name.len()..].trim_start();
        let after_for = after_name.strip_prefix("for ").ok_or_else(|| {
            CompileError::Syntax(format!(
                "security event `{name}` must use `security event Name for Model;`"
            ))
        })?;
        let end = after_for.find(';').ok_or_else(|| {
            CompileError::Syntax(format!("security event `{name}` must end with `;`"))
        })?;
        let object_raw = after_for[..end].trim();
        if !is_identifier(object_raw) {
            return Err(CompileError::Syntax(format!(
                "security event `{name}` has invalid model `{object_raw}`"
            )));
        }
        let object_type = resolve(namespace, object_raw);
        if program.model(&object_type).is_none() {
            return Err(CompileError::security(
                "SEC-A09-002",
                format!("security event `{name}` references unknown model `{object_raw}`"),
                Some("declare the model before the security event".into()),
            ));
        }
        let symbol = qualify(namespace, &name);
        if program.security_event(&symbol).is_some() {
            return Err(CompileError::Syntax(format!(
                "duplicate security event `{name}`"
            )));
        }
        program.security_events.push(SecurityEvent {
            name: symbol,
            object_type,
        });
        offset = source[keyword..]
            .find(';')
            .map(|v| keyword + v + 1)
            .unwrap_or(source.len());
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<(), CompileError> {
    if !is_identifier(name)
        || !name
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
    {
        return Err(CompileError::Syntax(format!(
            "security event `{name}` must start with an uppercase ASCII letter"
        )));
    }
    Ok(())
}
