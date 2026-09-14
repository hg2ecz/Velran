use crate::declarations;
use crate::diagnostics::CompileError;
use crate::module_namespace::{is_symbol_path, qualify};
use crate::query_mutation_contract::{parse_query_contracts, validate_sql_mutation_contract};
use crate::source_syntax::{matching_brace, matching_paren, read_ident, split_top_level};
use crate::sql_syntax::{first_sql_keyword, scan_bind_names};
use crate::type_resolution::resolve_annotated_value_type;
use language_core::{
    FunctionParam, Model, Program, QueryCapability, QueryFunction, QueryReturn, ValueType,
};
use std::collections::HashSet;

pub(super) fn parse_queries(
    source: &str,
    namespace: &str,
    p: &mut Program,
) -> Result<(), CompileError> {
    let mut off = 0;
    while let Some(rel) = source[off..].find("query fn ") {
        let keyword = off + rel;
        if !declarations::is_top_level_declaration_at(source, keyword) {
            off = keyword + 9;
            continue;
        }
        let start = keyword + 9;
        let name = read_ident(source, start)
            .ok_or_else(|| CompileError::Syntax("query name expected".into()))?;
        let symbol_name = qualify(namespace, &name);
        if p.queries.iter().any(|q| q.name == symbol_name) {
            return Err(CompileError::Syntax(format!("duplicate query `{name}`")));
        }
        let sig_open = source[start + name.len()..]
            .find('(')
            .map(|v| start + name.len() + v)
            .ok_or_else(|| CompileError::Syntax(format!("query `{name}` missing `(`")))?;
        let sig_close = matching_paren(source, sig_open)
            .ok_or_else(|| CompileError::Syntax(format!("query `{name}` signature unclosed")))?;
        let raw_params = split_top_level(&source[sig_open + 1..sig_close], ',');
        let mut capability = None;
        let mut params = Vec::new();
        for raw in raw_params {
            let raw = raw.trim();
            if raw.is_empty() {
                continue;
            }
            let (pn, pt) = raw.split_once(':').ok_or_else(|| {
                CompileError::Syntax(format!(
                    "query `{name}` parameter `{raw}` expected `name: Type`"
                ))
            })?;
            let pn = pn.trim();
            let pt = pt.trim();
            match pt {
                "Db" if pn == "db" && capability.is_none() => {
                    capability = Some(QueryCapability::Db)
                }
                "Transaction" if pn == "tx" && capability.is_none() => {
                    capability = Some(QueryCapability::Transaction)
                }
                _ => {
                    let annotated = resolve_annotated_value_type(pt, namespace, p)
                        .filter(|resolved| {
                            !matches!(
                                resolved.value_type,
                                ValueType::Upload
                                    | ValueType::F32Array
                                    | ValueType::StringList
                                    | ValueType::StringDict
                            )
                        })
                        .ok_or_else(|| {
                            CompileError::Syntax(format!(
                                "query `{name}` unsupported parameter type `{pt}`"
                            ))
                        })?;
                    params.push(FunctionParam {
                        name: pn.into(),
                        ty: annotated.value_type,
                        sensitivity: annotated.sensitivity,
                    });
                }
            }
        }
        let capability = capability.ok_or_else(|| {
            CompileError::Syntax(format!(
                "query `{name}` must begin with `db: Db` or `tx: Transaction`"
            ))
        })?;
        let after = &source[sig_close + 1..];
        let sql_rel = after
            .find("sql")
            .ok_or_else(|| CompileError::Syntax(format!("query `{name}` missing `sql` block")))?;
        let between = after[..sql_rel].trim();
        let declaration = between
            .strip_prefix("->")
            .ok_or_else(|| CompileError::Syntax(format!("query `{name}` missing return type")))?
            .trim();
        let contracts = parse_query_contracts(declaration, namespace, p, &params, &name)?;
        let return_type = resolve_query_return_models(
            parse_query_return(&contracts.return_type).ok_or_else(|| CompileError::Syntax(format!("query `{name}` requires `Result<(), DbError>`, `Result<Changed, DbError>`, `Result<Model, DbError>`, `Result<Option<Model>, DbError>`, or `Result<Vec<Model>, DbError>`")))?,
            namespace,
        );
        let model = match return_type.model_name() {
            Some(name) => Some(
                p.model(name)
                    .ok_or_else(|| CompileError::UnknownModel(name.into()))?,
            ),
            None => None,
        };
        let sql_kw = sig_close + 1 + sql_rel;
        let sql_open = source[sql_kw + 3..]
            .find('{')
            .map(|v| sql_kw + 3 + v)
            .ok_or_else(|| {
                CompileError::Syntax(format!("query `{name}` sql block missing `{{`"))
            })?;
        let sql_close = matching_brace(source, sql_open)
            .ok_or_else(|| CompileError::Syntax(format!("query `{name}` sql block unclosed")))?;
        let sql = source[sql_open + 1..sql_close].trim().to_string();
        if sql.to_ascii_lowercase().contains("_velran_") {
            return Err(CompileError::UnsafeSql(format!(
                "query `{name}` references reserved runtime database namespace `_velran_`"
            )));
        }
        validate_sql(&name, &sql, &params)?;
        let keyword = first_sql_keyword(&sql).unwrap_or_default();
        validate_sql_mutation_contract(
            &name,
            &keyword,
            &contracts.mutation_target,
            &contracts.credential_lifecycle,
            &return_type,
            &sql,
        )?;
        let tenant_scope = crate::tenant_security::infer_query_scope(
            &name,
            &return_type,
            contracts.mutation_target.as_ref(),
            contracts.credential_lifecycle.as_ref(),
            &params,
            &keyword,
            &sql,
            p,
        )?;
        let mutating = matches!(keyword.as_str(), "INSERT" | "UPDATE" | "DELETE");
        if mutating && capability != QueryCapability::Transaction {
            return Err(CompileError::UnsafeSql(format!(
                "mutating query `{name}` must require `tx: Transaction`"
            )));
        }
        if !mutating && capability != QueryCapability::Db {
            return Err(CompileError::UnsafeSql(format!(
                "read query `{name}` must require `db: Db`"
            )));
        }
        if !mutating && matches!(&return_type, &QueryReturn::Void | &QueryReturn::Changed) {
            return Err(CompileError::Syntax(format!(
                "read query `{name}` cannot return `()` or `Changed`"
            )));
        }
        if !mutating && matches!(&return_type, &QueryReturn::List(_)) {
            crate::query_resource_security::require_bounded_list_query(&name, &sql)?;
        }
        if mutating && matches!(&return_type, &QueryReturn::List(_)) {
            return Err(CompileError::Syntax(format!(
                "mutating query `{name}` cannot return `Vec<Model>`"
            )));
        }
        if let Some(model) = model {
            validate_returning_shape(&name, &sql, model, mutating)?;
        } else if mutating && sql.to_ascii_uppercase().contains("RETURNING") {
            return Err(CompileError::Syntax(format!(
                "`()`/`Changed` mutating query `{name}` must not use RETURNING"
            )));
        }
        p.queries.push(QueryFunction {
            name: symbol_name,
            capability,
            params,
            return_type,
            mutation_target: contracts.mutation_target,
            tenant_scope,
            credential_lifecycle: contracts.credential_lifecycle,
            sql,
        });
        off = sql_close + 1;
    }
    Ok(())
}

fn resolve_query_return_models(value: QueryReturn, namespace: &str) -> QueryReturn {
    match value {
        QueryReturn::One(name) => {
            QueryReturn::One(crate::module_namespace::resolve(namespace, &name))
        }
        QueryReturn::Optional(name) => {
            QueryReturn::Optional(crate::module_namespace::resolve(namespace, &name))
        }
        QueryReturn::List(name) => {
            QueryReturn::List(crate::module_namespace::resolve(namespace, &name))
        }
        QueryReturn::Void => QueryReturn::Void,
        QueryReturn::Changed => QueryReturn::Changed,
    }
}

fn parse_query_return(ret: &str) -> Option<QueryReturn> {
    let inner = ret.strip_prefix("Result<")?.strip_suffix('>')?;
    let parts = split_top_level(inner, ',');
    if parts.len() != 2 || parts[1].trim() != "DbError" {
        return None;
    }
    let value = parts[0].trim();
    if value == "()" {
        return Some(QueryReturn::Void);
    }
    if value == "Changed" {
        return Some(QueryReturn::Changed);
    }
    if let Some(inner) = value
        .strip_prefix("Option<")
        .and_then(|v| v.strip_suffix('>'))
    {
        return is_symbol_path(inner.trim()).then(|| QueryReturn::Optional(inner.trim().into()));
    }
    if let Some(inner) = value.strip_prefix("Vec<").and_then(|v| v.strip_suffix('>')) {
        return is_symbol_path(inner.trim()).then(|| QueryReturn::List(inner.trim().into()));
    }
    is_symbol_path(value).then(|| QueryReturn::One(value.into()))
}

fn validate_sql(name: &str, sql: &str, params: &[FunctionParam]) -> Result<(), CompileError> {
    if sql.contains("{{") || sql.contains("}}") || sql.contains("${") {
        return Err(CompileError::UnsafeSql(
            "SQL interpolation is forbidden; use `:name` binds".into(),
        ));
    }
    let binds = scan_bind_names(sql)?;
    let expected: HashSet<&str> = params.iter().map(|v| v.name.as_str()).collect();
    let actual: HashSet<&str> = binds.iter().map(String::as_str).collect();
    if expected != actual {
        return Err(CompileError::UnsafeSql(format!(
            "query `{name}` bind set does not exactly match typed parameters; expected {:?}, got {:?}",
            expected, actual
        )));
    }
    Ok(())
}

fn validate_returning_shape(
    name: &str,
    sql: &str,
    model: &Model,
    mutating: bool,
) -> Result<(), CompileError> {
    let upper = sql.to_ascii_uppercase();
    let marker = if mutating { "RETURNING" } else { "SELECT" };
    let pos = upper
        .find(marker)
        .ok_or_else(|| CompileError::Syntax(format!("query `{name}` must contain `{marker}`")))?
        + marker.len();
    let tail = &sql[pos..];
    let column_list = if mutating {
        tail.trim().to_string()
    } else {
        tail.split_whitespace()
            .take_while(|token| !token.eq_ignore_ascii_case("FROM"))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let cols: Vec<String> = column_list.split(',').map(projection_output_name).collect();
    let fields: Vec<String> = model.fields.iter().map(|v| v.name.clone()).collect();
    if cols != fields {
        return Err(CompileError::Syntax(format!(
            "query `{name}` selected/returned columns {:?} must exactly match model `{}` fields {:?} in order",
            cols, model.name, fields
        )));
    }
    Ok(())
}

fn projection_output_name(raw: &str) -> String {
    let tokens: Vec<&str> = raw.split_whitespace().collect();
    let output = if tokens.len() >= 3 && tokens[tokens.len() - 2].eq_ignore_ascii_case("AS") {
        tokens[tokens.len() - 1]
    } else {
        tokens.first().copied().unwrap_or("")
    };
    output.trim_matches('"').trim_matches('`').to_string()
}
