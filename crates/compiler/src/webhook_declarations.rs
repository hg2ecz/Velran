use crate::declarations;
use crate::diagnostics::CompileError;
use crate::module_namespace::qualify;
use crate::source_syntax::{is_identifier, matching_brace, read_ident};
use language_core::{Program, Webhook};

const MIN_REPLAY_WINDOW_SECS: u64 = 30;
const MAX_REPLAY_WINDOW_SECS: u64 = 3600;

pub(super) fn parse_webhooks(
    source: &str,
    namespace: &str,
    program: &mut Program,
) -> Result<(), CompileError> {
    let mut offset = 0usize;
    while let Some(relative) = source[offset..].find("webhook ") {
        let keyword = offset + relative;
        if !declarations::is_top_level_declaration_at(source, keyword) {
            offset = keyword + "webhook ".len();
            continue;
        }
        let start = keyword + "webhook ".len();
        let name = read_ident(source, start)
            .ok_or_else(|| CompileError::Syntax("webhook name expected".into()))?;
        validate_symbol_name(&name)?;
        let symbol = qualify(namespace, &name);
        if program.webhook(&symbol).is_some() {
            return Err(CompileError::Syntax(format!("duplicate webhook `{name}`")));
        }
        let open = skip_to_brace(source, start + name.len(), &name)?;
        let close = matching_brace(source, open)
            .ok_or_else(|| CompileError::Syntax(format!("webhook `{name}` body is unclosed")))?;
        let mut webhook = parse_body(&name, &source[open + 1..close])?;
        webhook.name = symbol;
        program.webhooks.push(webhook);
        offset = close + 1;
    }
    Ok(())
}

fn parse_body(name: &str, body: &str) -> Result<Webhook, CompileError> {
    let mut secret_name = None;
    let mut signature_header = None;
    let mut timestamp_header = None;
    let mut replay_window_secs = None;
    for line in body.lines() {
        let clean = line
            .split_once("//")
            .map_or(line, |(before, _)| before)
            .trim();
        let clean = clean.trim_end_matches(';').trim();
        if clean.is_empty() {
            continue;
        }
        if let Some(value) = clean.strip_prefix("verified by ") {
            if secret_name.is_some() {
                return Err(duplicate(name, "verified by"));
            }
            if !is_identifier(value) {
                return Err(CompileError::Syntax(format!(
                    "webhook `{name}` has invalid secret name `{value}`"
                )));
            }
            secret_name = Some(value.to_string());
        } else if let Some(value) = clean.strip_prefix("signatureHeader ") {
            if signature_header.is_some() {
                return Err(duplicate(name, "signatureHeader"));
            }
            signature_header = Some(parse_header_literal(name, "signatureHeader", value)?);
        } else if let Some(value) = clean.strip_prefix("timestampHeader ") {
            if timestamp_header.is_some() {
                return Err(duplicate(name, "timestampHeader"));
            }
            timestamp_header = Some(parse_header_literal(name, "timestampHeader", value)?);
        } else if let Some(value) = clean.strip_prefix("replayWindow ") {
            if replay_window_secs.is_some() {
                return Err(duplicate(name, "replayWindow"));
            }
            let seconds = value.parse::<u64>().map_err(|_| {
                CompileError::Syntax(format!(
                    "webhook `{name}` replayWindow must be whole seconds"
                ))
            })?;
            if !(MIN_REPLAY_WINDOW_SECS..=MAX_REPLAY_WINDOW_SECS).contains(&seconds) {
                return Err(CompileError::security(
                    "SEC-A08-021",
                    format!(
                        "webhook `{name}` replay window must be between {MIN_REPLAY_WINDOW_SECS} and {MAX_REPLAY_WINDOW_SECS} seconds"
                    ),
                    Some("use a narrow replay window such as `replayWindow 300`".into()),
                ));
            }
            replay_window_secs = Some(seconds);
        } else {
            return Err(CompileError::Syntax(format!(
                "webhook `{name}` entries are `verified by <SecretName>`, `signatureHeader \"...\"`, `timestampHeader \"...\"`, or `replayWindow <seconds>`"
            )));
        }
    }
    Ok(Webhook {
        name: String::new(),
        secret_name: secret_name.ok_or_else(|| CompileError::security(
            "SEC-A08-020",
            format!("webhook `{name}` has no verification secret"),
            Some("add `verified by <SecretName>`; the platform reads the secret from the configured webhook secrets directory".into()),
        ))?,
        signature_header: signature_header.unwrap_or_else(|| "x-velran-signature".into()),
        timestamp_header: timestamp_header.unwrap_or_else(|| "x-velran-timestamp".into()),
        replay_window_secs: replay_window_secs.unwrap_or(300),
    })
}

fn parse_header_literal(name: &str, field: &str, value: &str) -> Result<String, CompileError> {
    let value = value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .ok_or_else(|| {
            CompileError::Syntax(format!(
                "webhook `{name}` {field} must be a quoted HTTP header name"
            ))
        })?;
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(CompileError::security(
            "SEC-A05-060",
            format!("webhook `{name}` {field} is not a safe HTTP header name"),
            Some("use only ASCII letters, digits, and `-` in webhook header names".into()),
        ));
    }
    Ok(value.to_ascii_lowercase())
}

fn validate_symbol_name(name: &str) -> Result<(), CompileError> {
    if !is_identifier(name)
        || !name
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
    {
        return Err(CompileError::Syntax(format!(
            "webhook `{name}` must start with an uppercase ASCII letter"
        )));
    }
    Ok(())
}

fn skip_to_brace(source: &str, mut cursor: usize, name: &str) -> Result<usize, CompileError> {
    while source
        .as_bytes()
        .get(cursor)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        cursor += 1;
    }
    if source.as_bytes().get(cursor) != Some(&b'{') {
        return Err(CompileError::Syntax(format!(
            "webhook `{name}` must use `webhook Name {{ ... }}`"
        )));
    }
    Ok(cursor)
}

fn duplicate(name: &str, field: &str) -> CompileError {
    CompileError::Syntax(format!(
        "webhook `{name}` contains duplicate `{field}` entry"
    ))
}
