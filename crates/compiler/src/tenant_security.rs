use crate::diagnostics::CompileError;
use crate::expression_security::infer_static_expr_type;
use crate::handler_types::StaticType;
use crate::scalar_security::TenantEvidence;
use language_core::{
    CredentialLifecycleTarget, Expr, FunctionParam, MutationTarget, Program, QueryFunction,
    QueryReturn, TenantScopeTarget,
};
use std::collections::HashMap;

pub(super) fn infer_query_scope(
    query_name: &str,
    return_type: &QueryReturn,
    mutation_target: Option<&MutationTarget>,
    lifecycle_target: Option<&CredentialLifecycleTarget>,
    params: &[FunctionParam],
    sql_keyword: &str,
    sql: &str,
    program: &Program,
) -> Result<Option<TenantScopeTarget>, CompileError> {
    let return_model = return_type
        .model_name()
        .and_then(|name| program.model(name));
    let mutation_model = mutation_target.and_then(|target| program.model(&target.model));
    let lifecycle_model = lifecycle_target.and_then(|target| program.model(&target.model));
    let scoped = [return_model, mutation_model, lifecycle_model]
        .into_iter()
        .flatten()
        .filter_map(|model| model.tenant_field.as_ref().map(|field| (model, field)))
        .collect::<Vec<_>>();
    if scoped.is_empty() {
        return Ok(None);
    }
    let (model, field) = scoped[0];
    if scoped.iter().any(|(candidate, candidate_field)| {
        candidate.name != model.name || *candidate_field != field
    }) {
        return Err(CompileError::security(
            "SEC-A01-035",
            format!("query `{query_name}` crosses more than one tenant-scoped model"),
            Some("split cross-model tenant work into queries that preserve one tenant scope at a time".into()),
        ));
    }
    let model_field = model
        .fields
        .iter()
        .find(|candidate| candidate.name == *field)
        .ok_or_else(|| {
            CompileError::Syntax(format!(
                "internal: scoped model `{}` lost tenant field `{field}`",
                model.name
            ))
        })?;
    let param = params.iter().find(|candidate| candidate.name == *field).ok_or_else(|| {
        CompileError::security(
            "SEC-A01-036",
            format!("tenant-scoped query `{query_name}` must accept tenant parameter `{field}`"),
            Some(format!("add `{field}: <same type as {}.{field}>` to the query signature; pass the route tenant value unchanged", model.name)),
        )
    })?;
    if param.ty != model_field.ty {
        return Err(CompileError::security(
            "SEC-A01-037",
            format!("tenant parameter `{query_name}.{field}` must match `{}.{field}` exactly", model.name),
            Some("tenant identifiers are nominally typed and cannot be substituted with another String-like identifier".into()),
        ));
    }
    if !crate::tenant_sql::sql_has_tenant_guard(sql_keyword, sql, field) {
        return Err(CompileError::security(
            "SEC-A01-038",
            format!(
                "tenant-scoped query `{query_name}` does not constrain `{field}` with `:{field}`"
            ),
            Some(match sql_keyword {
                "INSERT" => {
                    format!("insert `{field}` from `:{field}` in the INSERT column/value pair")
                }
                _ => format!(
                    "add `WHERE ... {field} = :{field}` so the database operation is tenant-scoped"
                ),
            }),
        ));
    }
    Ok(Some(TenantScopeTarget {
        model: model.name.clone(),
        field: field.clone(),
    }))
}

pub(super) fn validate_query_call(
    query: &QueryFunction,
    args: &[Expr],
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<(), CompileError> {
    let Some(scope) = &query.tenant_scope else {
        return Ok(());
    };
    let index = query
        .params
        .iter()
        .position(|param| param.name == scope.field)
        .ok_or_else(|| {
            CompileError::Syntax(format!(
                "internal: query `{}` lost tenant parameter `{}`",
                query.name, scope.field
            ))
        })?;
    let argument = args.get(index).ok_or_else(|| {
        CompileError::Syntax(format!("query `{}` tenant argument is missing", query.name))
    })?;
    let tenant = infer_static_expr_type(argument, known, program)?
        .scalar()
        .and_then(|scalar| scalar.tenant);
    if tenant == Some(TenantEvidence::ActiveRouteTenant) {
        return Ok(());
    }
    Err(CompileError::security(
        "SEC-A01-039",
        format!(
            "tenant-scoped query `{}` requires the active route tenant for `{}`",
            query.name, scope.field
        ),
        Some(format!(
            "declare `tenant {}` on the authenticated route and pass that handler parameter unchanged to `{}`",
            scope.field, query.name
        )),
    ))
}
