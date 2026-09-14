use crate::diagnostics::CompileError;
use crate::source_syntax::is_identifier;
use language_core::PublicCachePolicy;

pub(super) fn parse_public_cache(
    tokens: &[String],
    cursor: &mut usize,
    route_name: &str,
) -> Result<Option<PublicCachePolicy>, CompileError> {
    if tokens.get(*cursor).map(String::as_str) != Some("cache") {
        return Ok(None);
    }
    if tokens.get(*cursor + 1).map(String::as_str) != Some("public")
        || tokens.get(*cursor + 2).map(String::as_str) != Some("ttl")
    {
        return Err(CompileError::Syntax(format!(
            "route `{route_name}` cache syntax is `cache public ttl <seconds>`"
        )));
    }
    let ttl_secs: u64 = tokens
        .get(*cursor + 3)
        .ok_or_else(|| CompileError::Syntax(format!("route `{route_name}` cache ttl expected")))?
        .parse()
        .map_err(|_| {
            CompileError::Syntax(format!("route `{route_name}` cache ttl must be integer"))
        })?;
    if ttl_secs == 0 {
        return Err(CompileError::Syntax(format!(
            "route `{route_name}` cache ttl must be > 0"
        )));
    }
    *cursor += 4;
    Ok(Some(PublicCachePolicy { ttl_secs }))
}

pub(super) fn parse_invalidations(
    tokens: &[String],
    cursor: &mut usize,
    route_name: &str,
) -> Result<Vec<String>, CompileError> {
    if tokens.get(*cursor).map(String::as_str) != Some("invalidate") {
        return Ok(Vec::new());
    }
    if tokens.get(*cursor + 1).map(String::as_str) != Some("cache") {
        return Err(CompileError::Syntax(format!(
            "route `{route_name}` invalidation syntax is `invalidate cache <route>...`"
        )));
    }
    *cursor += 2;
    let mut targets = Vec::new();
    while !matches!(tokens.get(*cursor).map(String::as_str), Some("=>") | None) {
        let target = tokens.get(*cursor).expect("cursor checked");
        if !is_identifier(target) {
            return Err(CompileError::Syntax(format!(
                "route `{route_name}` invalid cache route name `{target}`"
            )));
        }
        if targets.contains(target) {
            return Err(CompileError::Syntax(format!(
                "route `{route_name}` duplicate cache invalidation `{target}`"
            )));
        }
        targets.push(target.clone());
        *cursor += 1;
    }
    if targets.is_empty() {
        return Err(CompileError::Syntax(format!(
            "route `{route_name}` invalidate cache requires at least one route name"
        )));
    }
    Ok(targets)
}
