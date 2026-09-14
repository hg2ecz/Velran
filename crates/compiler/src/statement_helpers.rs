use crate::diagnostics::CompileError;
use crate::domain_symbols::{display_domain_symbol, internal_domain_symbol};
use crate::expression::{infer_expr_type, parse_expr_in_namespace, validate_expr};
use crate::handler_types::{StaticType, query_static_type};
use crate::module_namespace::{is_symbol_path, last_segment, resolve};
use crate::mutation_security::{validate_mutation_call, validate_mutation_inputs};
use crate::secret_usage::{reject_audit_secret, validate_query_argument};
use crate::source_syntax::{read_ident, split_top_level};
use crate::type_semantics::display;
use language_core::{BusinessAudit, Program, QueryCall, QueryCapability, ValueType};
use std::collections::HashMap;

fn audit_object_id_type_allowed(ty: ValueType) -> bool {
    !matches!(
        ty,
        ValueType::Image
            | ValueType::Upload
            | ValueType::F32Array
            | ValueType::StringList
            | ValueType::StringDict
    )
}
fn audit_change_type_allowed(ty: ValueType) -> bool {
    !matches!(
        ty,
        ValueType::String
            | ValueType::Email
            | ValueType::Url
            | ValueType::Image
            | ValueType::Upload
            | ValueType::F32Array
            | ValueType::StringList
            | ValueType::StringDict
    )
}

pub(super) fn parse_business_audit(
    handler: &str,
    namespace: &str,
    line: &str,
    known: &HashMap<String, StaticType>,
    p: &Program,
) -> Result<BusinessAudit, CompileError> {
    // Grammar:
    // audit <ObjectType> <object-id-expr> action <actionName>
    // audit <ObjectType> <object-id-expr> action <actionName> from <expr> to <expr>
    let rest = line
        .strip_prefix("audit ")
        .ok_or_else(|| CompileError::Syntax("internal audit parser error".into()))?
        .trim();
    let object_type_raw = rest.split_whitespace().next().ok_or_else(|| {
        CompileError::Syntax(format!("action `{handler}` audit object type expected"))
    })?;
    if !is_symbol_path(object_type_raw) {
        return Err(CompileError::Syntax(format!(
            "action `{handler}` audit object type `{object_type_raw}` is invalid"
        )));
    }
    let object_type = resolve(namespace, object_type_raw);
    if !last_segment(&object_type)
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_uppercase())
    {
        return Err(CompileError::Syntax(format!(
            "action `{handler}` audit object type `{object_type}` must start with an uppercase ASCII letter"
        )));
    }
    if p.model(&object_type).is_none() {
        return Err(CompileError::Syntax(format!(
            "action `{handler}` audit references unknown model/object `{object_type}`"
        )));
    }
    let after_object = rest[object_type_raw.len()..].trim_start();
    let action_pos = after_object.find(" action ").ok_or_else(|| CompileError::Syntax(format!("action `{handler}` audit syntax is `audit <ObjectType> <id> action <name> [from <old> to <new>]`")))?;
    let object_id_raw = after_object[..action_pos].trim();
    if object_id_raw.is_empty() {
        return Err(CompileError::Syntax(format!(
            "action `{handler}` audit object id expression expected"
        )));
    }
    let object_id = parse_expr_in_namespace(object_id_raw, namespace, p)?;
    validate_expr(&object_id, known, p)?;
    reject_audit_secret(&object_id, "business audit object id", known, p)?;
    let id_ty = infer_expr_type(&object_id, known, p)?;
    if !audit_object_id_type_allowed(id_ty) {
        return Err(CompileError::Syntax(format!(
            "action `{handler}` audit object id must be a scalar auditable type"
        )));
    }

    let action_and_change = after_object[action_pos + " action ".len()..].trim();
    let action = read_ident(action_and_change, 0).ok_or_else(|| {
        CompileError::Syntax(format!("action `{handler}` audit action name expected"))
    })?;
    let tail = action_and_change[action.len()..].trim();
    let (previous, new_value) = if tail.is_empty() {
        (None, None)
    } else {
        let change = tail.strip_prefix("from ").ok_or_else(|| {
            CompileError::Syntax(format!(
                "action `{handler}` audit change must use `from <old> to <new>`"
            ))
        })?;
        let to_pos = change.find(" to ").ok_or_else(|| {
            CompileError::Syntax(format!("action `{handler}` audit `from` requires `to`"))
        })?;
        let old_raw = change[..to_pos].trim();
        let new_raw = change[to_pos + 4..].trim();
        if old_raw.is_empty() || new_raw.is_empty() {
            return Err(CompileError::Syntax(format!(
                "action `{handler}` audit `from`/`to` expressions cannot be empty"
            )));
        }
        let old = parse_expr_in_namespace(old_raw, namespace, p)?;
        let new = parse_expr_in_namespace(new_raw, namespace, p)?;
        validate_expr(&old, known, p)?;
        validate_expr(&new, known, p)?;
        reject_audit_secret(&old, "business audit previous value", known, p)?;
        reject_audit_secret(&new, "business audit new value", known, p)?;
        let old_ty = infer_expr_type(&old, known, p)?;
        let new_ty = infer_expr_type(&new, known, p)?;
        if old_ty != new_ty {
            return Err(CompileError::Syntax(format!(
                "action `{handler}` audit `from` and `to` types must match"
            )));
        }
        if !audit_change_type_allowed(old_ty) {
            return Err(CompileError::Syntax(format!(
                "action `{handler}` audit values must be scalar auditable types"
            )));
        }
        (Some(old), Some(new))
    };
    Ok(BusinessAudit {
        object_type,
        object_id,
        action,
        previous,
        new_value,
        source_action: display_domain_symbol(handler),
    })
}

pub(super) fn parse_query_call(
    rhs: &str,
    namespace: &str,
    p: &Program,
    known: &HashMap<String, StaticType>,
    cap: QueryCapability,
) -> Result<Option<(QueryCall, StaticType)>, CompileError> {
    let rhs = rhs.trim().strip_suffix('?').unwrap_or(rhs.trim()).trim();
    let open = match rhs.find('(') {
        Some(v) => v,
        None => return Ok(None),
    };
    if !rhs.ends_with(')') {
        return Ok(None);
    }
    let source_qname = rhs[..open].trim();
    let Some(qname) = internal_domain_symbol(source_qname).map(|name| resolve(namespace, &name))
    else {
        return Ok(None);
    };
    let q = match p.query(&qname) {
        Some(v) => v,
        None => return Ok(None),
    };
    if q.capability != cap {
        return Err(CompileError::Syntax(format!(
            "query `{qname}` requires {:?}, not {:?}",
            q.capability, cap
        )));
    }
    let raw_args = split_top_level(&rhs[open + 1..rhs.len() - 1], ',');
    if raw_args.is_empty() {
        return Err(CompileError::Syntax(format!(
            "query call `{qname}` missing capability argument"
        )));
    }
    let expected_cap = match cap {
        QueryCapability::Db => "db",
        QueryCapability::Transaction => "tx",
    };
    if raw_args[0].trim() != expected_cap {
        return Err(CompileError::Syntax(format!(
            "query `{qname}` first argument must be `{expected_cap}`"
        )));
    }
    if raw_args.len() - 1 != q.params.len() {
        return Err(CompileError::Syntax(format!(
            "query `{qname}` expects {} value arguments",
            q.params.len()
        )));
    }
    let mut args = Vec::new();
    for (raw, param) in raw_args.iter().skip(1).zip(&q.params) {
        let e = parse_expr_in_namespace(raw.trim(), namespace, p)?;
        validate_expr(&e, known, p)?;
        let actual = infer_expr_type(&e, known, p)?;
        validate_query_argument(&qname, param, &e, known, p)?;
        if actual != param.ty {
            return Err(CompileError::security(
                "SEC-TYPE-001",
                format!(
                    "query `{qname}` argument `{}` expects `{}`, got `{}`",
                    param.name,
                    display(p, param.ty),
                    display(p, actual)
                ),
                Some("nominal domain types are not interchangeable even when they share the same String/Int representation".into()),
            ));
        }
        args.push(e);
    }
    crate::tenant_security::validate_query_call(q, &args, known, p)?;
    if cap == QueryCapability::Transaction {
        validate_mutation_inputs(q, &args, known, p)?;
    }
    validate_mutation_call(q, &args, known, p)?;
    Ok(Some((
        QueryCall {
            query: qname.into(),
            args,
        },
        query_static_type(q),
    )))
}

pub(super) fn parse_security_event(
    handler: &str,
    namespace: &str,
    line: &str,
    known: &HashMap<String, StaticType>,
    p: &Program,
) -> Result<BusinessAudit, CompileError> {
    let rest = line
        .strip_prefix("security ")
        .ok_or_else(|| CompileError::Syntax("internal security event parser error".into()))?
        .trim();
    let event_raw = rest.split_whitespace().next().ok_or_else(|| {
        CompileError::Syntax(format!("action `{handler}` security event name expected"))
    })?;
    if !is_symbol_path(event_raw) {
        return Err(CompileError::Syntax(format!(
            "action `{handler}` security event `{event_raw}` is invalid"
        )));
    }
    let event_name = resolve(namespace, event_raw);
    let event = p.security_event(&event_name).ok_or_else(|| {
        CompileError::security(
            "SEC-A09-003",
            format!("action `{handler}` references unknown security event `{event_raw}`"),
            Some("declare it with `security event Name for Model;`".into()),
        )
    })?;
    let tail = rest[event_raw.len()..].trim();
    let (object_id_raw, changes) = match tail.find(" from ") {
        Some(pos) => (&tail[..pos], Some(&tail[pos + " from ".len()..])),
        None => (tail, None),
    };
    if object_id_raw.trim().is_empty() {
        return Err(CompileError::Syntax(format!(
            "action `{handler}` security event object id expression expected"
        )));
    }
    let object_id = parse_expr_in_namespace(object_id_raw.trim(), namespace, p)?;
    validate_expr(&object_id, known, p)?;
    reject_audit_secret(&object_id, "security event object id", known, p)?;
    let id_ty = infer_expr_type(&object_id, known, p)?;
    if !audit_object_id_type_allowed(id_ty) {
        return Err(CompileError::Syntax(format!(
            "action `{handler}` security event object id must be a scalar auditable type"
        )));
    }
    let (previous, new_value) = if let Some(change) = changes {
        let to_pos = change.find(" to ").ok_or_else(|| {
            CompileError::Syntax(format!(
                "action `{handler}` security event change must use `from <old> to <new>`"
            ))
        })?;
        let old = parse_expr_in_namespace(change[..to_pos].trim(), namespace, p)?;
        let new = parse_expr_in_namespace(change[to_pos + 4..].trim(), namespace, p)?;
        validate_expr(&old, known, p)?;
        validate_expr(&new, known, p)?;
        reject_audit_secret(&old, "security event previous value", known, p)?;
        reject_audit_secret(&new, "security event new value", known, p)?;
        let old_ty = infer_expr_type(&old, known, p)?;
        let new_ty = infer_expr_type(&new, known, p)?;
        if old_ty != new_ty || !audit_change_type_allowed(old_ty) {
            return Err(CompileError::security(
                "SEC-A09-004",
                format!(
                    "security event `{event_raw}` from/to values must use the same auditable non-sensitive scalar type"
                ),
                None,
            ));
        }
        (Some(old), Some(new))
    } else {
        (None, None)
    };
    Ok(BusinessAudit {
        object_type: event.object_type.clone(),
        object_id,
        action: event
            .name
            .rsplit("::")
            .next()
            .unwrap_or(&event.name)
            .to_string(),
        previous,
        new_value,
        source_action: display_domain_symbol(handler),
    })
}
