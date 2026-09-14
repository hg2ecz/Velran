use crate::declarations;
use crate::diagnostics::CompileError;
use crate::module_namespace::qualify;
use crate::source_syntax::{matching_brace, read_ident};
use language_core::{DomainType, Program, ValueType};

mod constraints;

pub(super) fn parse_domain_types(
    source: &str,
    namespace: &str,
    program: &mut Program,
) -> Result<(), CompileError> {
    let mut offset = 0usize;
    while let Some(relative) = source[offset..].find("type ") {
        let keyword = offset + relative;
        if !declarations::is_top_level_declaration_at(source, keyword) {
            offset = keyword + 5;
            continue;
        }
        let name = read_ident(source, keyword + 5)
            .ok_or_else(|| CompileError::Syntax("domain type name expected".into()))?;
        require_type_name(&name)?;
        let after_name = keyword + 5 + name.len();
        let eq = skip_ascii_whitespace(source, after_name);
        if source.as_bytes().get(eq) != Some(&b'=') {
            return Err(CompileError::Syntax(format!("type `{name}` requires `=`")));
        }
        let base_start = skip_ascii_whitespace(source, eq + 1);
        let base_name = read_ident(source, base_start)
            .ok_or_else(|| CompileError::Syntax(format!("type `{name}` base type expected")))?;
        let open = skip_ascii_whitespace(source, base_start + base_name.len());
        if source.as_bytes().get(open) != Some(&b'{') {
            return Err(CompileError::Syntax(format!(
                "type `{name}` requires `{{` after its base type"
            )));
        }
        let close = matching_brace(source, open)
            .ok_or_else(|| CompileError::Syntax(format!("type `{name}` body is unclosed")))?;
        let base = ValueType::parse(&base_name).ok_or_else(|| {
            CompileError::Syntax(format!("type `{name}` base must be a built-in scalar type"))
        })?;
        if !matches!(base, ValueType::String | ValueType::Int) {
            return Err(CompileError::Syntax(format!(
                "type `{name}` currently supports String or Int bases"
            )));
        }
        let symbol = qualify(namespace, &name);
        if program.domain_type(&symbol).is_some() {
            return Err(CompileError::Syntax(format!("duplicate type `{name}`")));
        }
        let mut constraints = constraints::parse(&name, base, &source[open + 1..close])?;
        constraints::apply_safe_defaults(base, &mut constraints);
        program.domain_types.push(DomainType {
            name: symbol,
            base,
            constraints,
        });
        offset = close + 1;
    }
    Ok(())
}

fn require_type_name(name: &str) -> Result<(), CompileError> {
    if name
        .chars()
        .next()
        .is_some_and(|value| value.is_ascii_uppercase())
    {
        Ok(())
    } else {
        Err(CompileError::Syntax(format!(
            "domain type `{name}` must start with an uppercase ASCII letter"
        )))
    }
}

fn skip_ascii_whitespace(source: &str, mut offset: usize) -> usize {
    while source
        .as_bytes()
        .get(offset)
        .is_some_and(|value| value.is_ascii_whitespace())
    {
        offset += 1;
    }
    offset
}
