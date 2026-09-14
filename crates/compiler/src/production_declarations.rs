use crate::declarations;
use crate::diagnostics::CompileError;
use crate::source_syntax::matching_brace;
use language_core::{ProductionPolicy, Program};

pub(super) fn parse_production_policy(
    source: &str,
    program: &mut Program,
) -> Result<(), CompileError> {
    let mut offset = 0usize;
    while let Some(relative) = source[offset..].find("production") {
        let keyword = offset + relative;
        if !declarations::is_top_level_declaration_at(source, keyword)
            || !keyword_boundary(source, keyword + "production".len())
        {
            offset = keyword + "production".len();
            continue;
        }
        if program.production.is_some() {
            return Err(CompileError::Syntax(
                "only one `production { ... }` policy may be declared across the application"
                    .into(),
            ));
        }
        let open = find_open_brace(source, keyword + "production".len())?;
        let close = matching_brace(source, open)
            .ok_or_else(|| CompileError::Syntax("production policy body is unclosed".into()))?;
        parse_body(&source[open + 1..close])?;
        program.production = Some(ProductionPolicy::STRICT);
        offset = close + 1;
    }
    Ok(())
}

fn keyword_boundary(source: &str, end: usize) -> bool {
    source
        .as_bytes()
        .get(end)
        .map_or(true, |byte| !byte.is_ascii_alphanumeric() && *byte != b'_')
}

fn find_open_brace(source: &str, mut cursor: usize) -> Result<usize, CompileError> {
    while source
        .as_bytes()
        .get(cursor)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        cursor += 1;
    }
    if source.as_bytes().get(cursor) != Some(&b'{') {
        return Err(CompileError::Syntax(
            "production policy must use `production { ... }`".into(),
        ));
    }
    Ok(cursor)
}

fn parse_body(body: &str) -> Result<(), CompileError> {
    let mut https = false;
    let mut debug = false;
    let mut hsts = false;
    let mut db_tls = false;
    for raw in body.split(';') {
        let entry = raw
            .lines()
            .map(|line| line.split_once("//").map_or(line, |(before, _)| before))
            .collect::<Vec<_>>()
            .join(" ");
        let entry = entry.split_whitespace().collect::<Vec<_>>().join(" ");
        if entry.is_empty() {
            continue;
        }
        let slot = match entry.as_str() {
            "https required" => &mut https,
            "debug disabled" => &mut debug,
            "hsts required" => &mut hsts,
            "database tls required" => &mut db_tls,
            _ => {
                return Err(CompileError::security(
                    "SEC-PROD-001",
                    format!("unknown or insecure production policy entry `{entry}`"),
                    Some("use exactly `https required;`, `debug disabled;`, `hsts required;`, and `database tls required;`".into()),
                ));
            }
        };
        if *slot {
            return Err(CompileError::Syntax(format!(
                "duplicate production policy entry `{entry}`"
            )));
        }
        *slot = true;
    }
    if !(https && debug && hsts && db_tls) {
        return Err(CompileError::security(
            "SEC-PROD-002",
            "production policy is incomplete",
            Some("declare all four strict invariants: `https required; debug disabled; hsts required; database tls required;`".into()),
        ));
    }
    Ok(())
}
