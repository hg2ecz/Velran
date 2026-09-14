#![allow(unused_imports)]

use crate::cache_safety::{
    action_has_business_audit, action_has_object_auth, validate_public_cache_statements,
};
use crate::expression::{infer_expr_type, infer_static_expr_type, parse_expr, validate_expr};
use crate::expression_token::ExprToken;
use crate::handler_types::StaticType;
use crate::lexer::tokenize;
use crate::source_syntax::{
    consume_return_tail, find_statement_end, function_bounds, is_identifier, line_number,
    matching_brace, matching_paren, preview, read_ident, skip_ws_and_comments, split_top_level,
};
use crate::sql_syntax::{first_sql_keyword, scan_bind_names};
use crate::type_resolution::resolve_value_type;
use crate::{CompileError, compile_file, compile_file_with_dependencies, compile_source};
use language_core::{
    ActionBody, ActionFunction, ActionStatement, AuthorizationMode, BinaryOp, BuiltinFunction,
    BusinessAudit, ComponentFunction, ComputeStatement, Expr, FlashKind, FlashMessage,
    HtmlAttrKind, HtmlPart, HtmlTemplate, HttpMethod, LayoutFunction, ObjectAuthorization,
    PageBody, PageFunction, Program, PublicCachePolicy, QueryCall, QueryCapability, QueryFunction,
    QueryReturn, ResourceUse, Route, RouteAuth, RouteSegment, SourceLocation, Statement,
    TemplateParam, TemplateParamType, TxStatement, UploadField, ValidationKind, ValueType,
};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
mod arrays_tests;
mod builtin_tests;
mod core_compile_tests;
mod data_contract_compile_tests;
mod domain_compile_tests;
mod effect_security_tests;
mod f32_tests;
mod fuzz_regression_tests;
mod idempotency_compile_tests;
mod inherent_impl_compile_tests;
mod integration_effect_tests;
mod legacy_rejection_tests;
mod media_compile_tests;
mod module_namespace_compile_tests;
mod module_namespace_visibility_compile_tests;
mod module_visibility_compile_tests;
mod numeric_string_core_tests;
mod package_compile_tests;
mod presentation_compile_tests;
mod production_policy_tests;
mod public_error_boundary_tests;
mod redaction_security_tests;
mod resource_safety_tests;
mod route_budget_compile_tests;
mod security_language_tests;
mod statement_terminator_tests;
mod sum_type_match_tests;
mod tenant_isolation_tests;
mod transaction_outcome_tests;
mod typed_json_response_tests;
mod visibility_compile_tests;
mod web_flow_compile_tests;
mod webhook_compile_tests;
