use crate::{DataError, DbBackend, DbValue};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct PreparedSql {
    source: String,
    ordered_names: Vec<String>,
}

impl PreparedSql {
    /// Builds a SQL template from compiler-owned SQL text.
    ///
    /// Application/user values must never be concatenated into `source`.
    /// Only `:name` markers outside SQL strings/comments are accepted as binds.
    pub fn compile(source: impl Into<String>) -> Result<Self, DataError> {
        let source = source.into();
        reject_multi_statement(&source)?;
        let ordered_names = scan_bind_names(&source)?;
        Ok(Self {
            source,
            ordered_names,
        })
    }

    pub fn bind_names(&self) -> &[String] {
        &self.ordered_names
    }

    pub(crate) fn render_for(&self, backend: DbBackend) -> Result<String, DataError> {
        rewrite_named_binds(&self.source, backend)
    }
}

#[derive(Debug, Clone)]
pub struct BindSet {
    values: HashMap<String, DbValue>,
}

impl BindSet {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: impl Into<String>, value: DbValue) -> Result<(), DataError> {
        let name = name.into();
        validate_bind_name(&name)?;
        if self.values.insert(name.clone(), value).is_some() {
            return Err(DataError::DuplicateBind(name));
        }
        Ok(())
    }

    pub(crate) fn ordered<'a>(&'a self, sql: &PreparedSql) -> Result<Vec<&'a DbValue>, DataError> {
        let expected: HashSet<&str> = sql.ordered_names.iter().map(String::as_str).collect();
        for supplied in self.values.keys() {
            if !expected.contains(supplied.as_str()) {
                return Err(DataError::UnexpectedBind(supplied.clone()));
            }
        }
        sql.ordered_names
            .iter()
            .map(|name| {
                self.values
                    .get(name)
                    .ok_or_else(|| DataError::MissingBind(name.clone()))
            })
            .collect()
    }
}

impl Default for BindSet {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_bind_name(name: &str) -> Result<(), DataError> {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c == '_' || c.is_ascii_alphabetic() => {}
        _ => return Err(DataError::InvalidBindName(name.into())),
    }
    if !chars.all(|c| c == '_' || c.is_ascii_alphanumeric()) {
        return Err(DataError::InvalidBindName(name.into()));
    }
    Ok(())
}

fn reject_multi_statement(sql: &str) -> Result<(), DataError> {
    // Semicolons outside strings/comments are rejected. v0.1 query fn == exactly one statement.
    if scan_sql(sql, ScanMode::RejectSemicolon)?.saw_semicolon {
        return Err(DataError::MultipleStatements);
    }
    Ok(())
}

fn scan_bind_names(sql: &str) -> Result<Vec<String>, DataError> {
    Ok(scan_sql(sql, ScanMode::CollectBinds)?.bind_names)
}

fn rewrite_named_binds(sql: &str, backend: DbBackend) -> Result<String, DataError> {
    let scan = scan_sql(sql, ScanMode::Rewrite(backend))?;
    Ok(scan.rewritten.unwrap_or_default())
}

#[derive(Clone, Copy)]
enum ScanMode {
    RejectSemicolon,
    CollectBinds,
    Rewrite(DbBackend),
}

struct ScanResult {
    bind_names: Vec<String>,
    saw_semicolon: bool,
    rewritten: Option<String>,
}

fn scan_sql(sql: &str, mode: ScanMode) -> Result<ScanResult, DataError> {
    let bytes = sql.as_bytes();
    let mut i = 0usize;
    let mut bind_names = Vec::new();
    let mut saw_semicolon = false;
    let mut out = matches!(mode, ScanMode::Rewrite(_)).then(|| String::with_capacity(sql.len()));
    let mut pg_index = 1usize;

    while i < bytes.len() {
        match bytes[i] {
            b'\'' => {
                let start = i;
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\'' {
                        if i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                            i += 2;
                            continue;
                        }
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                if i > bytes.len() || bytes.get(i.saturating_sub(1)) != Some(&b'\'') {
                    return Err(DataError::MalformedSql);
                }
                if let Some(o) = out.as_mut() {
                    o.push_str(&sql[start..i]);
                }
            }
            b'"' => {
                let start = i;
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'"' {
                        if i + 1 < bytes.len() && bytes[i + 1] == b'"' {
                            i += 2;
                            continue;
                        }
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                if bytes.get(i.saturating_sub(1)) != Some(&b'"') {
                    return Err(DataError::MalformedSql);
                }
                if let Some(o) = out.as_mut() {
                    o.push_str(&sql[start..i]);
                }
            }
            b'`' => {
                let start = i;
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'`' {
                        if i + 1 < bytes.len() && bytes[i + 1] == b'`' {
                            i += 2;
                            continue;
                        }
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                if bytes.get(i.saturating_sub(1)) != Some(&b'`') {
                    return Err(DataError::MalformedSql);
                }
                if let Some(o) = out.as_mut() {
                    o.push_str(&sql[start..i]);
                }
            }
            b'[' => {
                let start = i;
                i += 1;
                while i < bytes.len() && bytes[i] != b']' {
                    i += 1;
                }
                if i >= bytes.len() {
                    return Err(DataError::MalformedSql);
                }
                i += 1;
                if let Some(o) = out.as_mut() {
                    o.push_str(&sql[start..i]);
                }
            }
            b'$' if bytes.get(i + 1) == Some(&b'$')
                || bytes
                    .get(i + 1)
                    .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_') =>
            {
                return Err(DataError::UnsupportedSqlSyntax);
            }
            b'-' if bytes.get(i + 1) == Some(&b'-') => {
                let start = i;
                i += 2;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                if i < bytes.len() {
                    i += 1;
                }
                if let Some(o) = out.as_mut() {
                    o.push_str(&sql[start..i]);
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                let start = i;
                i += 2;
                let mut closed = false;
                while i + 1 < bytes.len() {
                    if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                        i += 2;
                        closed = true;
                        break;
                    }
                    i += 1;
                }
                if !closed {
                    return Err(DataError::MalformedSql);
                }
                if let Some(o) = out.as_mut() {
                    o.push_str(&sql[start..i]);
                }
            }
            b';' => {
                saw_semicolon = true;
                if let Some(o) = out.as_mut() {
                    o.push(';');
                }
                i += 1;
            }
            b':' if bytes.get(i + 1) == Some(&b':') => {
                if let Some(o) = out.as_mut() {
                    o.push_str("::");
                }
                i += 2;
            }
            b':' => {
                let start_name = i + 1;
                let first = bytes.get(start_name).copied();
                if !matches!(first, Some(b'_') | Some(b'a'..=b'z') | Some(b'A'..=b'Z')) {
                    if let Some(o) = out.as_mut() {
                        o.push(':');
                    }
                    i += 1;
                    continue;
                }
                let mut end = start_name + 1;
                while let Some(b) = bytes.get(end) {
                    if b.is_ascii_alphanumeric() || *b == b'_' {
                        end += 1;
                    } else {
                        break;
                    }
                }
                let name = &sql[start_name..end];
                validate_bind_name(name)?;
                bind_names.push(name.to_string());
                if let Some(o) = out.as_mut() {
                    match mode {
                        ScanMode::Rewrite(DbBackend::PostgreSql) => {
                            o.push('$');
                            o.push_str(&pg_index.to_string());
                            pg_index += 1;
                        }
                        ScanMode::Rewrite(DbBackend::Sqlite) => {
                            o.push('$');
                            o.push_str(&pg_index.to_string());
                            pg_index += 1;
                        }
                        ScanMode::Rewrite(DbBackend::MariaDb) => o.push('?'),
                        _ => o.push_str(&sql[i..end]),
                    }
                }
                i = end;
            }
            _ => {
                if let Some(o) = out.as_mut() {
                    let ch = sql[i..].chars().next().ok_or(DataError::MalformedSql)?;
                    o.push(ch);
                    i += ch.len_utf8();
                } else {
                    i += 1;
                }
            }
        }
    }

    Ok(ScanResult {
        bind_names,
        saw_semicolon,
        rewritten: out,
    })
}
