use crate::diagnostics::CompileError;
use crate::source_syntax::is_identifier;
use language_core::{Program, ResourceUse, SourceLocation};

pub(super) fn parse_route_budget(
    tokens: &[String],
    cursor: &mut usize,
    route_name: &str,
    namespace: &str,
    line: usize,
    program: &mut Program,
) -> Result<Option<String>, CompileError> {
    if tokens.get(*cursor).map(String::as_str) != Some("budget") {
        return Ok(None);
    }

    let profile = tokens
        .get(*cursor + 1)
        .ok_or_else(|| {
            CompileError::Syntax(format!("route `{route_name}` budget profile expected"))
        })?
        .clone();
    if !is_identifier(&profile) {
        return Err(CompileError::Syntax(format!(
            "route `{route_name}` invalid budget profile name"
        )));
    }
    if profile == "default" {
        return Err(CompileError::security(
            "SEC-A10-002",
            format!(
                "route `{route_name}` cannot request the implicit `default` budget profile"
            ),
            Some(
                "remove `budget default`; every request already runs under the platform default budget"
                    .to_string(),
            ),
        ));
    }

    program.resource_uses.push(ResourceUse {
        profile: profile.clone(),
        source: SourceLocation {
            file: namespace.to_string(),
            line,
            function: route_name.to_string(),
        },
    });
    *cursor += 2;
    Ok(Some(profile))
}
