use crate::diagnostics::CompileError;
const FORBIDDEN: &[&str] = &[
    "object ",
    "model ",
    "permission ",
    "critical ",
    "webhook ",
    "integration ",
    "production ",
    "security event ",
    "query fn ",
    "form ",
    "component fn ",
    "layout fn ",
    "page fn ",
    "action fn ",
    "route ",
];
pub(crate) fn validate(source: &str) -> Result<(), CompileError> {
    let mut offset = 0usize;
    for (i, raw) in source.split_inclusive('\n').enumerate() {
        let t = raw.trim_start();
        let pos = offset + (raw.len() - t.len());
        let line = t.trim();
        if !line.is_empty() && crate::declarations::is_top_level_declaration_at(source, pos) {
            let d = line.strip_prefix("pub ").unwrap_or(line);
            if FORBIDDEN.iter().any(|p| d.starts_with(p)) {
                return Err(CompileError::security("SEC-PKG-001",format!("line {}: local libraries may not declare web/server authority",i+1),Some("keep routes/pages/actions/queries and capability-bearing declarations in the application package".to_string())));
            }
        }
        offset += raw.len()
    }
    Ok(())
}
