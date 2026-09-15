use super::{
    CODEGEN_VERSION, GeneratedRust, RUNTIME_ABI_VERSION, STATUS_BAD_REQUEST,
    STATUS_BUDGET_EXCEEDED, STATUS_INTERNAL, STATUS_MEMORY_EXCEEDED, STATUS_OK,
    STATUS_OUTPUT_TOO_SMALL, STATUS_UNSUPPORTED, VALUE_BOOL, VALUE_HTML, VALUE_INT, VALUE_NONE,
    VALUE_TYPED_JSON, capability_name, crate_ident, lower,
};
use executable_ir::NativeInputType;
use executable_ir::shard_planner::VerifiedShard;
use language_core::{InlineHint, PureParamType};

pub(crate) fn generate_source(shard: &VerifiedShard) -> GeneratedRust {
    let crate_name = format!("velran_shard_{}", crate_ident(shard.id().as_str()));
    let mut source = String::new();
    source.push_str("pub mod implementation {\n#![forbid(unsafe_code)]\n#![deny(warnings)]\n\n// Generated only from VerifiedShard. Do not edit.\nuse super::VelranHostApi;\n");
    source.push_str(&format!(
        "pub const VELRAN_MODULE_ABI_VERSION: u32 = {RUNTIME_ABI_VERSION};\n"
    ));
    source.push_str(&format!(
        "pub const VELRAN_CODEGEN_VERSION: &str = {:?};\n",
        CODEGEN_VERSION
    ));
    source.push_str(&format!(
        "pub const VELRAN_LANGUAGE_VERSION: &str = {:?};\n",
        shard.language_version()
    ));
    source.push_str(&format!(
        "pub const VELRAN_SECURITY_POLICY_VERSION: &str = {:?};\n",
        shard.security_policy_version()
    ));
    source.push_str(&format!(
        "pub const VELRAN_EXECUTABLE_IR_VERSION: u16 = {};\n",
        shard.executable_ir_version()
    ));
    source.push_str(&format!(
        "pub const VELRAN_RUNTIME_CONTRACT_FINGERPRINT: u64 = {};\n",
        crate::runtime_contract_fingerprint(shard)
    ));
    source.push_str(&format!("pub const VELRAN_STATUS_OK: u32 = {STATUS_OK};\npub const VELRAN_STATUS_INTERNAL: u32 = {STATUS_INTERNAL};\npub const VELRAN_STATUS_BUDGET_EXCEEDED: u32 = {STATUS_BUDGET_EXCEEDED};\npub const VELRAN_STATUS_MEMORY_EXCEEDED: u32 = {STATUS_MEMORY_EXCEEDED};\npub const VELRAN_STATUS_BAD_REQUEST: u32 = {STATUS_BAD_REQUEST};\npub const VELRAN_STATUS_OUTPUT_TOO_SMALL: u32 = {STATUS_OUTPUT_TOO_SMALL};\npub const VELRAN_STATUS_UNSUPPORTED: u32 = {STATUS_UNSUPPORTED};\npub const VELRAN_MAX_PURE_CALL_DEPTH: u16 = 64;\n"));
    source.push_str(&format!("pub const VELRAN_VALUE_NONE: u32 = {VALUE_NONE};\npub const VELRAN_VALUE_INT: u32 = {VALUE_INT};\npub const VELRAN_VALUE_BOOL: u32 = {VALUE_BOOL};\npub const VELRAN_VALUE_F32_INTERNAL: u32 = 0x8000_0001;\npub const VELRAN_VALUE_HTML: u32 = {VALUE_HTML};\npub const VELRAN_VALUE_TYPED_JSON: u32 = {VALUE_TYPED_JSON};\n\n"));
    source.push_str("pub const VELRAN_CAPABILITIES: &[&str] = &[\n");
    for capability in shard.capabilities() {
        source.push_str(&format!("    {:?},\n", capability_name(capability)));
    }
    source.push_str("];\n\n");
    source.push_str("pub const VELRAN_HANDLER_NAMES: &[&str] = &[\n");
    for handler_name in shard.handlers().keys() {
        source.push_str(&format!("    {:?},\n", handler_name));
    }
    source.push_str("];\n\n");
    source.push_str("#[derive(Clone, Copy)]\npub enum InputValue<'a> {\n    Int(i64),\n    Bool(bool),\n    String(&'a str),\n    Email(&'a str),\n    Url(&'a str),\n    Slug(&'a str),\n    DomainInt(u16, i64),\n    DomainBool(u16, bool),\n    DomainString(u16, &'a str),\n    Upload(&'a [u8]),\n    Image(&'a [u8]),\n}\n\n");
    emit_pure_struct_types(&mut source, shard);
    source.push_str(SUPPORT_SOURCE);
    emit_pure_struct_field_accessor(&mut source, shard);
    source.push_str(super::numeric_support::SOURCE);

    for function in shard.pure_functions().values() {
        let inline = match function.inline() {
            InlineHint::Never => "#[inline(never)]\n",
            InlineHint::Always => "#[inline(always)]\n",
            InlineHint::Hint => "#[inline]\n",
            InlineHint::Unspecified => "",
        };
        source.push_str(inline);
        source.push_str("#[allow(unused_variables, unused_mut)]\n");
        if let Some(numeric_body) = function.numeric_body() {
            let params = function
                .params()
                .iter()
                .zip(numeric_body.inputs())
                .map(|((_, ty), input)| {
                    let PureParamType::F32ArrayMut(size) = *ty else {
                        unreachable!("numeric pure helper has only mutable f32 arrays")
                    };
                    format!(
                        "{}: &mut [f32; {size}]",
                        super::typed_lower::local_ident(input.local)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            let comma = if params.is_empty() { "" } else { ", " };
            source.push_str(&format!("fn {}({params}{comma}instruction_budget: u64, allocation_budget: u64) -> (u32, u32, u64, u64, u64, u64) {{\n", super::pure_function_ident(function.name())));
            source.push_str("    let mut velran_fuel = Fuel::new(instruction_budget);\n    let mut velran_state = RuntimeState::new(allocation_budget);\n");
            source.push_str(&format!(
                "    // Velran verified pure function: {:?}; host_api=false; ambient_authority=false\n",
                function.name()
            ));
            let lowered = super::typed_lower::body(numeric_body);
            for line in lowered.lines() {
                source.push_str("    ");
                source.push_str(line);
                source.push('\n');
            }
            if function.return_type() == language_core::PureReturnType::Unit {
                source.push_str("    typed_result_bool(true, &velran_fuel, &velran_state)\n");
            }
            source.push_str("}\n\n");
        } else {
            let params = function
                .params()
                .iter()
                .enumerate()
                .map(|(index, (_, ty))| match ty {
                    PureParamType::Int => format!("velran_arg_{index}: i64"),
                    PureParamType::Bool => format!("velran_arg_{index}: bool"),
                    PureParamType::Str => format!("velran_arg_{index}: std::sync::Arc<str>"),
                    PureParamType::StringList => {
                        format!("velran_arg_{index}: std::sync::Arc<Vec<std::sync::Arc<str>>>")
                    }
                    PureParamType::Struct(_) => format!("velran_arg_{index}: PureStructValue"),
                    PureParamType::F32ArrayMut(_) => {
                        unreachable!("nonnumeric pure helper cannot contain mutable f32 arrays")
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            let comma = if params.is_empty() { "" } else { ", " };
            source.push_str(&format!("fn {}({params}{comma}instruction_budget: u64, allocation_budget: u64, recursion_budget: u16) -> PureScalarResult {{\n", super::pure_function_ident(function.name())));
            source.push_str("    let mut velran_fuel = Fuel::new(instruction_budget);\n    let mut velran_state = RuntimeState::new(allocation_budget);\n");
            source.push_str("    if recursion_budget == 0 { return pure_budget_exceeded(&velran_fuel, &velran_state); }\n");
            source.push_str("    let velran_recursion_budget = recursion_budget;\n");
            source.push_str(&format!("    // Velran verified owned scalar pure function: {:?}; host_api=false; ambient_authority=false\n", function.name()));
            for (index, (name, ty)) in function.params().iter().enumerate() {
                match ty {
                    PureParamType::Int => source.push_str(&format!(
                        "    let mut {} = Scalar::Int(velran_arg_{index});\n",
                        super::local_ident(name)
                    )),
                    PureParamType::Bool => source.push_str(&format!(
                        "    let mut {} = Scalar::Bool(velran_arg_{index});\n",
                        super::local_ident(name)
                    )),
                    PureParamType::Str => source.push_str(&format!(
                        "    let mut {} = Scalar::String(velran_arg_{index});\n",
                        super::local_ident(name)
                    )),
                    PureParamType::StringList => source.push_str(&format!(
                        "    let mut {} = Scalar::StringList(velran_arg_{index});\n",
                        super::local_ident(name)
                    )),
                    PureParamType::Struct(_) => source.push_str(&format!(
                        "    let mut {} = Scalar::Struct(velran_arg_{index});\n",
                        super::local_ident(name)
                    )),
                    PureParamType::F32ArrayMut(_) => {
                        unreachable!("nonnumeric pure helper parameter contract")
                    }
                }
            }
            let lowered = super::lower::pure_body(function.body());
            for line in lowered.lines() {
                source.push_str("    ");
                source.push_str(line);
                source.push('\n');
            }
            if function.return_type() == language_core::PureReturnType::Unit {
                source.push_str("    pure_ok_unit(&velran_fuel, &velran_state)\n");
            }
            source.push_str("}\n\n");
        }
    }

    for (handler_id, (handler_name, handler)) in shard.handlers().iter().enumerate() {
        let body = handler
            .native_scalar_body()
            .expect("generate() verified native lowering");
        let numeric_body = body.numeric_body();
        if let Some(numeric_body) = numeric_body {
            // The ABI adapter decodes InputValue exactly once. The hot kernel itself has only
            // primitive typed parameters and therefore cannot observe the generic request ABI.
            source.push_str(&format!("#[allow(unused_variables, unused_mut)]\nfn velran_handler_{handler_id}(inputs: &[InputValue<'_>], output: &mut [u8], instruction_budget: u64, allocation_budget: u64) -> (u32, u32, u64, u64, u64, u64) {{\n"));
            source.push_str(&format!("    if inputs.len() != {} {{ return (VELRAN_STATUS_BAD_REQUEST, VELRAN_VALUE_NONE, 0, 0, 0, 0); }}\n", body.inputs().len()));
            for (index, input) in numeric_body.inputs().iter().enumerate() {
                let binder = direct_typed_input_binder(&input.param.ty, index);
                let ty =
                    typed_input_rust_type(&input.param.ty).expect("verified numeric input type");
                source.push_str(&format!("    let {}: {ty} = match {binder} {{ Some(v) => v, None => return (VELRAN_STATUS_BAD_REQUEST, VELRAN_VALUE_NONE, 0, 0, 0, 0) }};\n", super::typed_lower::local_ident(input.local)));
            }
            let args = numeric_body
                .inputs()
                .iter()
                .map(|input| super::typed_lower::local_ident(input.local))
                .collect::<Vec<_>>()
                .join(", ");
            source.push_str(&format!("    velran_kernel_{handler_id}({args}{}output, instruction_budget, allocation_budget)\n", if args.is_empty() { "" } else { ", " }));
            source.push_str("}\n\n");

            let params = numeric_body
                .inputs()
                .iter()
                .map(|input| {
                    let ty = typed_input_rust_type(&input.param.ty)
                        .expect("verified numeric input type");
                    format!("mut {}: {ty}", super::typed_lower::local_ident(input.local))
                })
                .collect::<Vec<_>>()
                .join(", ");
            source.push_str(&format!("#[inline(never)]\n#[allow(unused_variables, unused_mut)]\nfn velran_kernel_{handler_id}({params}{}output: &mut [u8], instruction_budget: u64, allocation_budget: u64) -> (u32, u32, u64, u64, u64, u64) {{\n", if params.is_empty() { "" } else { ", " }));
            source.push_str("    let mut velran_fuel = Fuel::new(instruction_budget);\n    let mut velran_state = RuntimeState::new(allocation_budget);\n    let mut velran_output = OutputWriter::new(output);\n");
            source.push_str(&format!(
                "    // Velran pure typed kernel: {:?}; generic_input_abi=false; host_api=false\n",
                handler_name
            ));
            let lowered = super::typed_lower::body(numeric_body);
            for line in lowered.lines() {
                source.push_str("    ");
                source.push_str(line);
                source.push('\n');
            }
            source.push_str("}\n\n");
        } else if super::typed_scalar_lower::eligible(body) {
            source.push_str(&format!("#[allow(unused_variables, unused_mut)]\nfn velran_handler_{handler_id}(inputs: &[InputValue<'_>], output: &mut [u8], instruction_budget: u64, allocation_budget: u64) -> (u32, u32, u64, u64, u64, u64) {{\n"));
            source.push_str(&format!("    if inputs.len() != {} {{ return (VELRAN_STATUS_BAD_REQUEST, VELRAN_VALUE_NONE, 0, 0, 0, 0); }}\n", body.inputs().len()));
            source.push_str("    let mut velran_fuel = Fuel::new(instruction_budget);\n    let mut velran_state = RuntimeState::new(allocation_budget);\n    let mut velran_output = OutputWriter::new(output);\n");
            for (index, input) in body.inputs().iter().enumerate() {
                let binder = super::typed_scalar_lower::input_binder(&input.ty, index);
                let ty = body
                    .local_type(&input.name)
                    .expect("verified input local type");
                source.push_str(&format!("    let mut {}: {} = match {binder} {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }};\n", super::local_ident(&input.name), super::typed_scalar_lower::rust_type(ty)));
            }
            source.push_str(&format!(
                "    // Velran direct typed scalar kernel: {:?}; scalar_enum=false; host_api=false\n",
                handler_name
            ));
            let lowered = super::typed_scalar_lower::body(body);
            for line in lowered.lines() {
                source.push_str("    ");
                source.push_str(line);
                source.push('\n');
            }
            source.push_str("}\n\n");
        } else {
            let uses_host_api = body.uses_host_api();
            let host_param = if uses_host_api {
                "velran_host: &HostApi, "
            } else {
                ""
            };
            source.push_str(&format!("#[allow(unused_variables, unused_mut)]\nfn velran_handler_{handler_id}({host_param}inputs: &[InputValue<'_>], output: &mut [u8], instruction_budget: u64, allocation_budget: u64) -> (u32, u32, u64, u64, u64, u64) {{\n"));
            source.push_str(&format!("    if inputs.len() != {} {{ return (VELRAN_STATUS_BAD_REQUEST, VELRAN_VALUE_NONE, 0, 0, 0, 0); }}\n", body.inputs().len()));
            source.push_str("    let mut velran_fuel = Fuel::new(instruction_budget);\n    let mut velran_state = RuntimeState::new(allocation_budget);\n    let mut velran_output = OutputWriter::new(output);\n");
            for (index, input) in body.inputs().iter().enumerate() {
                let binder = input_binder(&input.ty, index);
                source.push_str(&format!("    let mut {} = match {binder} {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }};\n", super::local_ident(&input.name)));
            }
            source.push_str(&format!("    // Velran handler: {:?}; typed_numeric_fast_path=false; direct_typed_scalar=false; host_api={}\n", handler_name, uses_host_api));
            let lowered = lower::body(body);
            for line in lowered.lines() {
                source.push_str("    ");
                source.push_str(line);
                source.push('\n');
            }
            source.push_str("}\n\n");
        }
    }

    source.push_str("#[allow(unused_variables)]\npub fn velran_invoke_safe(handler_id: u32, host: &VelranHostApi, inputs: &[InputValue<'_>], output: &mut [u8], instruction_budget: u64, allocation_budget: u64) -> (u32, u32, u64, u64, u64, u64) {\n    match handler_id {\n");
    for (handler_id, (_, handler)) in shard.handlers().iter().enumerate() {
        let body = handler
            .native_scalar_body()
            .expect("generate() verified native lowering");
        if body.numeric_body().is_some() || !body.uses_host_api() {
            source.push_str(&format!("        {handler_id} => velran_handler_{handler_id}(inputs, output, instruction_budget, allocation_budget),\n"));
        } else {
            source.push_str(&format!("        {handler_id} => {{ let velran_host = HostApi::new(host); velran_handler_{handler_id}(&velran_host, inputs, output, instruction_budget, allocation_budget) }},\n"));
        }
    }
    source.push_str(
        "        _ => (VELRAN_STATUS_BAD_REQUEST, VELRAN_VALUE_NONE, 0, 0, 0, 0),\n    }\n}\n",
    );
    source.push_str("}\n");
    GeneratedRust { crate_name, source }
}
fn emit_pure_struct_types(source: &mut String, shard: &VerifiedShard) {
    source.push_str("#[derive(Clone)]\n#[allow(dead_code)]\npub(crate) enum PureStructValue {\n");
    if shard.struct_schemas().is_empty() {
        source.push_str("    __Unused,\n");
    }
    for (id, schema) in shard.struct_schemas() {
        source.push_str(&format!("    S{id} {{ "));
        for (index, field) in schema.fields.iter().enumerate() {
            let ty = match field.ty {
                language_core::ValueType::Int => "i64",
                language_core::ValueType::F32 => "f32",
                language_core::ValueType::Bool => "bool",
                language_core::ValueType::String => "std::sync::Arc<str>",
                language_core::ValueType::StringList => "std::sync::Arc<Vec<std::sync::Arc<str>>>",
                _ => unreachable!("verified pure struct field type"),
            };
            source.push_str(&format!("f{index}: {ty}, "));
        }
        source.push_str("},\n");
    }
    source.push_str("}\n\n");
}

fn emit_pure_struct_field_accessor(source: &mut String, shard: &VerifiedShard) {
    source.push_str("pub(crate) fn pure_struct_field_value(value: &PureStructValue, field: &str, state: &mut RuntimeState) -> Option<Scalar> { match (value, field) {\n");
    for (id, schema) in shard.struct_schemas() {
        for (index, field) in schema.fields.iter().enumerate() {
            let binders = (0..schema.fields.len())
                .map(|i| {
                    if i == index {
                        format!("f{i}")
                    } else {
                        format!("f{i}: _")
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            let wrap = match field.ty {
                language_core::ValueType::Int => format!("Scalar::Int(*f{index})"),
                language_core::ValueType::F32 => format!("Scalar::F32(*f{index})"),
                language_core::ValueType::Bool => format!("Scalar::Bool(*f{index})"),
                language_core::ValueType::String => format!("Scalar::String(f{index}.clone())"),
                language_core::ValueType::StringList => {
                    format!("Scalar::StringList(f{index}.clone())")
                }
                _ => unreachable!("verified pure struct field type"),
            };
            source.push_str(&format!(
                "    (PureStructValue::S{id} {{ {binders} }}, {:?}) => Some({wrap}),\n",
                field.name
            ));
        }
    }
    source.push_str("    _ => { state.bad_request(); None }\n} }\n\n");
}

fn input_binder(ty: &NativeInputType, index: usize) -> String {
    match ty {
        NativeInputType::F32Array | NativeInputType::StringList | NativeInputType::Struct(_) => {
            unreachable!("pure-function internal refs never cross the handler ABI")
        }
        NativeInputType::Int => format!("input_int(&inputs[{index}], &mut velran_state)"),
        NativeInputType::Bool => format!("input_bool(&inputs[{index}], &mut velran_state)"),
        NativeInputType::String => format!("input_string(&inputs[{index}], &mut velran_state)"),
        NativeInputType::Email => format!("input_email(&inputs[{index}], &mut velran_state)"),
        NativeInputType::Url => format!("input_url(&inputs[{index}], &mut velran_state)"),
        NativeInputType::Slug => format!("input_slug(&inputs[{index}], &mut velran_state)"),
        NativeInputType::DomainInt { domain, ranges } => {
            let checks = ranges
                .iter()
                .map(|(min, max)| format!("({min}i64,{max}i64)"))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "input_domain_int(&inputs[{index}], {domain}u16, &[{checks}], &mut velran_state)"
            )
        }
        NativeInputType::DomainBool { domain } => {
            format!("input_domain_bool(&inputs[{index}], {domain}u16, &mut velran_state)")
        }
        NativeInputType::Upload => format!("input_upload(&inputs[{index}], &mut velran_state)"),
        NativeInputType::Image => format!("input_image(&inputs[{index}], &mut velran_state)"),
        NativeInputType::DomainString { domain, lengths } => {
            let checks = lengths
                .iter()
                .map(|(min, max)| format!("({min}usize,{max}usize)"))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "input_domain_string(&inputs[{index}], {domain}u16, &[{checks}], &mut velran_state)"
            )
        }
    }
}

fn typed_input_rust_type(ty: &NativeInputType) -> Option<&'static str> {
    match ty {
        NativeInputType::F32Array | NativeInputType::StringList | NativeInputType::Struct(_) => {
            None
        }
        NativeInputType::Int | NativeInputType::DomainInt { .. } => Some("i64"),
        NativeInputType::Bool | NativeInputType::DomainBool { .. } => Some("bool"),
        _ => None,
    }
}
fn direct_typed_input_binder(ty: &NativeInputType, index: usize) -> String {
    match ty {
        NativeInputType::F32Array | NativeInputType::StringList | NativeInputType::Struct(_) => {
            unreachable!("pure-function internal refs never cross the handler ABI")
        }
        NativeInputType::Int => {
            format!("match &inputs[{index}] {{ InputValue::Int(v) => Some(*v), _ => None }}")
        }
        NativeInputType::Bool => {
            format!("match &inputs[{index}] {{ InputValue::Bool(v) => Some(*v), _ => None }}")
        }
        NativeInputType::DomainInt { domain, ranges } => {
            let checks = if ranges.is_empty() {
                "true".to_owned()
            } else {
                ranges
                    .iter()
                    .map(|(min, max)| format!("(*v >= {min}i64 && *v <= {max}i64)"))
                    .collect::<Vec<_>>()
                    .join(" || ")
            };
            format!(
                "match &inputs[{index}] {{ InputValue::DomainInt(actual, v) if *actual == {domain}u16 && ({checks}) => Some(*v), _ => None }}"
            )
        }
        NativeInputType::DomainBool { domain } => format!(
            "match &inputs[{index}] {{ InputValue::DomainBool(actual, v) if *actual == {domain}u16 => Some(*v), _ => None }}"
        ),
        _ => unreachable!("verified pure numeric input type"),
    }
}

const SUPPORT_SOURCE: &str = r#"
#[allow(dead_code)]
mod velran_support {
use super::{
    InputValue, PureStructValue, VELRAN_MODULE_ABI_VERSION, VELRAN_STATUS_BAD_REQUEST, VELRAN_STATUS_BUDGET_EXCEEDED, VELRAN_STATUS_INTERNAL,
    VELRAN_STATUS_MEMORY_EXCEEDED, VELRAN_STATUS_OK, VELRAN_STATUS_OUTPUT_TOO_SMALL, VELRAN_VALUE_BOOL,
    VELRAN_VALUE_F32_INTERNAL, VELRAN_VALUE_HTML, VELRAN_VALUE_INT, VELRAN_VALUE_NONE, VELRAN_VALUE_TYPED_JSON,
};
use crate::{VelranHostApi, VelranHostResult};
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::sync::Arc;

#[derive(Clone)]
#[allow(dead_code)]
pub(crate) enum PureSumValue {
    OptionIntSome(i64),
    OptionIntNone,
    OptionF32Some(f32),
    OptionF32None,
    OptionBoolSome(bool),
    OptionBoolNone,
    OptionStringSome(Arc<str>),
    OptionStringNone,
    OptionStringListSome(Arc<Vec<Arc<str>>>),
    OptionStringListNone,
    ResultIntIntOk(i64),
    ResultIntIntErr(i64),
    ResultIntF32Ok(i64),
    ResultIntF32Err(f32),
    ResultIntBoolOk(i64),
    ResultIntBoolErr(bool),
    ResultIntStringOk(i64),
    ResultIntStringErr(Arc<str>),
    ResultIntStringListOk(i64),
    ResultIntStringListErr(Arc<Vec<Arc<str>>>),
    ResultF32IntOk(f32),
    ResultF32IntErr(i64),
    ResultF32F32Ok(f32),
    ResultF32F32Err(f32),
    ResultF32BoolOk(f32),
    ResultF32BoolErr(bool),
    ResultF32StringOk(f32),
    ResultF32StringErr(Arc<str>),
    ResultF32StringListOk(f32),
    ResultF32StringListErr(Arc<Vec<Arc<str>>>),
    ResultBoolIntOk(bool),
    ResultBoolIntErr(i64),
    ResultBoolF32Ok(bool),
    ResultBoolF32Err(f32),
    ResultBoolBoolOk(bool),
    ResultBoolBoolErr(bool),
    ResultBoolStringOk(bool),
    ResultBoolStringErr(Arc<str>),
    ResultBoolStringListOk(bool),
    ResultBoolStringListErr(Arc<Vec<Arc<str>>>),
    ResultStringIntOk(Arc<str>),
    ResultStringIntErr(i64),
    ResultStringF32Ok(Arc<str>),
    ResultStringF32Err(f32),
    ResultStringBoolOk(Arc<str>),
    ResultStringBoolErr(bool),
    ResultStringStringOk(Arc<str>),
    ResultStringStringErr(Arc<str>),
    ResultStringStringListOk(Arc<str>),
    ResultStringStringListErr(Arc<Vec<Arc<str>>>),
    ResultStringListIntOk(Arc<Vec<Arc<str>>>),
    ResultStringListIntErr(i64),
    ResultStringListF32Ok(Arc<Vec<Arc<str>>>),
    ResultStringListF32Err(f32),
    ResultStringListBoolOk(Arc<Vec<Arc<str>>>),
    ResultStringListBoolErr(bool),
    ResultStringListStringOk(Arc<Vec<Arc<str>>>),
    ResultStringListStringErr(Arc<str>),
    ResultStringListStringListOk(Arc<Vec<Arc<str>>>),
    ResultStringListStringListErr(Arc<Vec<Arc<str>>>),
}

#[derive(Clone)]
pub(crate) enum Scalar { Int(i64), F32(f32), F32Array(Vec<f32>), Bool(bool), String(Arc<str>), StringList(Arc<Vec<Arc<str>>>), StringDict(Arc<BTreeMap<Arc<str>, Arc<str>>>), Struct(PureStructValue), PureSum(PureSumValue), Upload(UploadValue), Image(ImageValue) }
#[derive(Clone)] pub(crate) struct UploadValue { path: Arc<str>, filename: Arc<str>, content_type: Arc<str>, bytes: i64 }
#[derive(Clone)] pub(crate) struct ImageValue { path: Arc<str>, content_type: Arc<str>, width: i64, height: i64, bytes: i64 }

pub(crate) struct Fuel { initial: u64, remaining: u64 }
impl Fuel {
    pub(crate) fn new(limit: u64) -> Self { Self { initial: limit, remaining: limit } }
    pub(crate) fn charge(&mut self, amount: u64) -> bool { match self.remaining.checked_sub(amount) { Some(left) => { self.remaining = left; true }, None => false } }
    pub(crate) fn used(&self) -> u64 { self.initial - self.remaining }
    pub(crate) fn remaining(&self) -> u64 { self.remaining }
}

pub(crate) struct HostApi { context: *mut c_void, call: Option<extern "C" fn(*mut c_void,u32,*const u8,u64,*mut u8,u64)->VelranHostResult>, abi_version: u32 }
impl HostApi {
    pub(crate) fn new(raw: &VelranHostApi) -> Self { Self { context: raw.context, call: raw.call, abi_version: raw.abi_version } }
    pub(crate) fn outbound_status(&self, method: u8, target: &str, path: &str, body: Option<&[u8]>, state: &mut RuntimeState) -> Option<i64> {
        if self.abi_version != VELRAN_MODULE_ABI_VERSION { state.bad_request(); return None; }
        let call = self.call?;
        let body = body.unwrap_or(&[]);
        let target_len = u32::try_from(target.len()).ok()?; let path_len = u32::try_from(path.len()).ok()?; let body_len = u32::try_from(body.len()).ok()?;
        let total = 1usize.checked_add(4)?.checked_add(target.len())?.checked_add(4)?.checked_add(path.len())?.checked_add(4)?.checked_add(body.len())?;
        if total > 1_048_576 || !state.charge_alloc(total as u64) { return None; }
        let mut request = Vec::with_capacity(total); request.push(method); request.extend_from_slice(&target_len.to_le_bytes()); request.extend_from_slice(target.as_bytes()); request.extend_from_slice(&path_len.to_le_bytes()); request.extend_from_slice(path.as_bytes()); request.extend_from_slice(&body_len.to_le_bytes()); request.extend_from_slice(body);
        let operation = if method == 0 { 1u32 } else { 2u32 };
        let result = call(self.context, operation, request.as_ptr(), request.len() as u64, std::ptr::null_mut(), 0);
        if result.status == 0 && result.value_tag == 1 && result.output_len == 0 { Some(result.payload as i64) } else { None }
    }
}

pub(crate) fn scalar_json_body(value: Option<Scalar>, state: &mut RuntimeState) -> Option<Option<Vec<u8>>> {
    let value = value?;
    let text = match value {
        Scalar::Int(v) => v.to_string(),
        Scalar::Bool(v) => if v { "true".to_owned() } else { "false".to_owned() },
        Scalar::String(v) => { let mut s=String::with_capacity(v.len()+2); s.push('\"'); for ch in v.chars() { match ch { '\"'=>s.push_str("\\\""), '\\'=>s.push_str("\\\\"), '\n'=>s.push_str("\\n"), '\r'=>s.push_str("\\r"), '\t'=>s.push_str("\\t"), c if c.is_control()=>return None, c=>s.push(c) } } s.push('\"'); s },
        _ => { state.bad_request(); return None; }
    };
    if !state.charge_alloc(text.len() as u64) { return None; }
    Some(Some(text.into_bytes()))
}

pub(crate) struct RuntimeState { remaining_alloc: u64, allocated: u64, failure: Option<u32> }
impl RuntimeState {
    pub(crate) fn new(limit: u64) -> Self { Self { remaining_alloc: limit, allocated: 0, failure: None } }
    pub(crate) fn charge_alloc(&mut self, amount: u64) -> bool {
        match self.remaining_alloc.checked_sub(amount) {
            Some(left) => { self.remaining_alloc = left; self.allocated = self.allocated.saturating_add(amount); true }
            None => { self.failure = Some(VELRAN_STATUS_MEMORY_EXCEEDED); false }
        }
    }
    pub(crate) fn bad_request(&mut self) { if self.failure.is_none() { self.failure = Some(VELRAN_STATUS_BAD_REQUEST); } }
    pub(crate) fn memory_exceeded(&mut self) { if self.failure.is_none() { self.failure = Some(VELRAN_STATUS_MEMORY_EXCEEDED); } }
    pub(crate) fn remaining_alloc(&self) -> u64 { self.remaining_alloc }
    pub(crate) fn allocated(&self) -> u64 { self.allocated }
}

pub(crate) struct PureScalarResult {
    pub(crate) status: u32,
    pub(crate) value: Option<Scalar>,
    pub(crate) fuel_used: u64,
    pub(crate) allocated: u64,
}
pub(crate) fn pure_result(value: Scalar, fuel: &Fuel, state: &RuntimeState) -> PureScalarResult {
    let valid = !matches!(&value, Scalar::F32(v) if !v.is_finite());
    if valid {
        PureScalarResult { status: VELRAN_STATUS_OK, value: Some(value), fuel_used: fuel.used(), allocated: state.allocated }
    } else {
        PureScalarResult { status: VELRAN_STATUS_INTERNAL, value: None, fuel_used: fuel.used(), allocated: state.allocated }
    }
}
pub(crate) fn pure_fail(fuel: &Fuel, state: &RuntimeState) -> PureScalarResult {
    PureScalarResult { status: state.failure.unwrap_or(VELRAN_STATUS_INTERNAL), value: None, fuel_used: fuel.used(), allocated: state.allocated }
}
pub(crate) fn pure_budget_exceeded(fuel: &Fuel, state: &RuntimeState) -> PureScalarResult {
    PureScalarResult { status: VELRAN_STATUS_BUDGET_EXCEEDED, value: None, fuel_used: fuel.used(), allocated: state.allocated }
}

pub(crate) fn pure_ok_unit(fuel: &Fuel, state: &RuntimeState) -> PureScalarResult {
    PureScalarResult { status: VELRAN_STATUS_OK, value: None, fuel_used: fuel.used(), allocated: state.allocated }
}

pub(crate) fn result(value: Scalar, fuel: &Fuel, state: &RuntimeState) -> (u32, u32, u64, u64, u64, u64) {
    match value {
        Scalar::Int(v) => (VELRAN_STATUS_OK, VELRAN_VALUE_INT, v as u64, fuel.used(), state.allocated, 0),
        Scalar::Bool(v) => (VELRAN_STATUS_OK, VELRAN_VALUE_BOOL, v as u64, fuel.used(), state.allocated, 0),
        Scalar::F32(v) if v.is_finite() => (VELRAN_STATUS_OK, VELRAN_VALUE_F32_INTERNAL, u64::from(v.to_bits()), fuel.used(), state.allocated, 0),
        Scalar::F32(_) | Scalar::F32Array(_) | Scalar::String(_) | Scalar::StringList(_) | Scalar::StringDict(_) | Scalar::Struct(_) | Scalar::PureSum(_) | Scalar::Upload(_) | Scalar::Image(_) => (VELRAN_STATUS_INTERNAL, VELRAN_VALUE_NONE, 0, fuel.used(), state.allocated, 0),
    }
}
pub(crate) fn fail(fuel: &Fuel, state: &RuntimeState) -> (u32, u32, u64, u64, u64, u64) { (state.failure.unwrap_or(VELRAN_STATUS_INTERNAL), VELRAN_VALUE_NONE, 0, fuel.used(), state.allocated, 0) }
pub(crate) fn budget_exceeded(fuel: &Fuel, state: &RuntimeState) -> (u32, u32, u64, u64, u64, u64) { (VELRAN_STATUS_BUDGET_EXCEEDED, VELRAN_VALUE_NONE, 0, fuel.used(), state.allocated, 0) }
pub(crate) fn input_int(value: &InputValue<'_>, state: &mut RuntimeState) -> Option<Scalar> {
    if let InputValue::Int(v) = value { Some(Scalar::Int(*v)) } else { state.bad_request(); None }
}
pub(crate) fn input_bool(value: &InputValue<'_>, state: &mut RuntimeState) -> Option<Scalar> {
    if let InputValue::Bool(v) = value { Some(Scalar::Bool(*v)) } else { state.bad_request(); None }
}
pub(crate) fn input_string(value: &InputValue<'_>, state: &mut RuntimeState) -> Option<Scalar> {
    if let InputValue::String(v) = value { alloc_string(v, state) } else { state.bad_request(); None }
}
pub(crate) fn input_email(value: &InputValue<'_>, state: &mut RuntimeState) -> Option<Scalar> {
    if let InputValue::Email(v) = value { alloc_string(v, state) } else { state.bad_request(); None }
}
pub(crate) fn input_url(value: &InputValue<'_>, state: &mut RuntimeState) -> Option<Scalar> {
    if let InputValue::Url(v) = value { alloc_string(v, state) } else { state.bad_request(); None }
}
pub(crate) fn input_slug(value: &InputValue<'_>, state: &mut RuntimeState) -> Option<Scalar> {
    if let InputValue::Slug(v) = value { alloc_string(v, state) } else { state.bad_request(); None }
}
pub(crate) fn input_domain_int(value: &InputValue<'_>, expected: u16, ranges: &[(i64, i64)], state: &mut RuntimeState) -> Option<Scalar> {
    let InputValue::DomainInt(domain, v) = value else { state.bad_request(); return None; };
    if *domain != expected || !ranges.iter().all(|(min, max)| v >= min && v <= max) { state.bad_request(); return None; }
    Some(Scalar::Int(*v))
}
pub(crate) fn input_domain_bool(value: &InputValue<'_>, expected: u16, state: &mut RuntimeState) -> Option<Scalar> {
    let InputValue::DomainBool(domain, v) = value else { state.bad_request(); return None; };
    if *domain != expected { state.bad_request(); return None; }
    Some(Scalar::Bool(*v))
}
pub(crate) fn input_domain_string(value: &InputValue<'_>, expected: u16, lengths: &[(usize, usize)], state: &mut RuntimeState) -> Option<Scalar> {
    let InputValue::DomainString(domain, v) = value else { state.bad_request(); return None; };
    let chars = v.chars().count();
    if *domain != expected || !lengths.iter().all(|(min, max)| chars >= *min && chars <= *max) { state.bad_request(); return None; }
    alloc_string(v, state)
}

pub(crate) fn input_upload(value: &InputValue<'_>, state: &mut RuntimeState) -> Option<Scalar> {
    let InputValue::Upload(raw) = value else { state.bad_request(); return None; };
    decode_upload(raw, state).map(Scalar::Upload)
}
pub(crate) fn input_image(value: &InputValue<'_>, state: &mut RuntimeState) -> Option<Scalar> {
    let InputValue::Image(raw) = value else { state.bad_request(); return None; };
    decode_image(raw, state).map(Scalar::Image)
}
fn read_u32(raw: &[u8], at: &mut usize) -> Option<u32> { let end=at.checked_add(4)?; let b:[u8;4]=raw.get(*at..end)?.try_into().ok()?; *at=end; Some(u32::from_le_bytes(b)) }
fn read_u64(raw: &[u8], at: &mut usize) -> Option<u64> { let end=at.checked_add(8)?; let b:[u8;8]=raw.get(*at..end)?.try_into().ok()?; *at=end; Some(u64::from_le_bytes(b)) }
fn read_text(raw: &[u8], at: &mut usize, max: usize, state: &mut RuntimeState) -> Option<Arc<str>> { let len=usize::try_from(read_u32(raw, at)?).ok()?; if len>max { state.bad_request(); return None; } let end=at.checked_add(len)?; let v=std::str::from_utf8(raw.get(*at..end)?).ok()?; *at=end; alloc_string(v,state).and_then(as_string) }
fn decode_upload(raw: &[u8], state: &mut RuntimeState) -> Option<UploadValue> { let mut at=0; if *raw.first()?!=1 { state.bad_request(); return None; } at+=1; let path=read_text(raw,&mut at,4096,state)?; let filename=read_text(raw,&mut at,4096,state)?; let content_type=read_text(raw,&mut at,256,state)?; let bytes=i64::try_from(read_u64(raw,&mut at)?).ok()?; if at!=raw.len(){state.bad_request();return None;} Some(UploadValue{path,filename,content_type,bytes}) }
fn decode_image(raw: &[u8], state: &mut RuntimeState) -> Option<ImageValue> { let mut at=0; if *raw.first()?!=1 { state.bad_request(); return None; } at+=1; let path=read_text(raw,&mut at,4096,state)?; let content_type=read_text(raw,&mut at,256,state)?; let width=i64::from(read_u32(raw,&mut at)?); let height=i64::from(read_u32(raw,&mut at)?); let bytes=i64::try_from(read_u64(raw,&mut at)?).ok()?; if at!=raw.len(){state.bad_request();return None;} Some(ImageValue{path,content_type,width,height,bytes}) }
pub(crate) fn field_value(value: &Scalar, field: &str, state: &mut RuntimeState) -> Option<Scalar> { match (value,field) {
 (Scalar::Upload(v),"path")=>Some(Scalar::String(v.path.clone())), (Scalar::Upload(v),"filename")=>Some(Scalar::String(v.filename.clone())), (Scalar::Upload(v),"contentType")=>Some(Scalar::String(v.content_type.clone())), (Scalar::Upload(v),"bytes")=>Some(Scalar::Int(v.bytes)),
 (Scalar::Image(v),"path")=>Some(Scalar::String(v.path.clone())), (Scalar::Image(v),"contentType")=>Some(Scalar::String(v.content_type.clone())), (Scalar::Image(v),"width")=>Some(Scalar::Int(v.width)), (Scalar::Image(v),"height")=>Some(Scalar::Int(v.height)), (Scalar::Image(v),"bytes")=>Some(Scalar::Int(v.bytes)),
 (Scalar::Struct(v),field)=>super::pure_struct_field_value(v,field,state), _=>{state.bad_request();None} } }

pub(crate) fn as_int(value: Scalar) -> Option<i64> { if let Scalar::Int(v) = value { Some(v) } else { None } }
pub(crate) fn as_bool(value: Scalar) -> Option<bool> { if let Scalar::Bool(v) = value { Some(v) } else { None } }
pub(crate) fn as_f32(value: Scalar) -> Option<f32> { if let Scalar::F32(v) = value { Some(v) } else { None } }
pub(crate) fn as_string(value: Scalar) -> Option<Arc<str>> { if let Scalar::String(v) = value { Some(v) } else { None } }
pub(crate) fn non_negative_usize(value: Scalar, state: &mut RuntimeState) -> Option<usize> {
    let value = as_int(value)?;
    match usize::try_from(value) { Ok(v) => Some(v), Err(_) => { state.bad_request(); None } }
}
pub(crate) fn typed_input_int(value: &InputValue<'_>, state: &mut RuntimeState) -> Option<i64> {
    if let InputValue::Int(v) = value { Some(*v) } else { state.bad_request(); None }
}
pub(crate) fn typed_input_bool(value: &InputValue<'_>, state: &mut RuntimeState) -> Option<bool> {
    if let InputValue::Bool(v) = value { Some(*v) } else { state.bad_request(); None }
}
pub(crate) fn typed_input_domain_int(value: &InputValue<'_>, expected: u16, ranges: &[(i64, i64)], state: &mut RuntimeState) -> Option<i64> {
    let InputValue::DomainInt(domain, v) = value else { state.bad_request(); return None; };
    if *domain != expected || !ranges.iter().all(|(min, max)| v >= min && v <= max) { state.bad_request(); return None; }
    Some(*v)
}
pub(crate) fn typed_input_domain_bool(value: &InputValue<'_>, expected: u16, state: &mut RuntimeState) -> Option<bool> {
    let InputValue::DomainBool(domain, v) = value else { state.bad_request(); return None; };
    if *domain != expected { state.bad_request(); return None; }
    Some(*v)
}
pub(crate) fn typed_result_i64(value: i64, fuel: &Fuel, state: &RuntimeState) -> (u32,u32,u64,u64,u64,u64) { (VELRAN_STATUS_OK, VELRAN_VALUE_INT, value as u64, fuel.used(), state.allocated, 0) }
pub(crate) fn typed_result_bool(value: bool, fuel: &Fuel, state: &RuntimeState) -> (u32,u32,u64,u64,u64,u64) { (VELRAN_STATUS_OK, VELRAN_VALUE_BOOL, value as u64, fuel.used(), state.allocated, 0) }
pub(crate) fn typed_result_f32(value: f32, fuel: &Fuel, state: &RuntimeState) -> (u32,u32,u64,u64,u64,u64) { if value.is_finite() { (VELRAN_STATUS_OK, VELRAN_VALUE_F32_INTERNAL, u64::from(value.to_bits()), fuel.used(), state.allocated, 0) } else { (VELRAN_STATUS_INTERNAL, VELRAN_VALUE_NONE, 0, fuel.used(), state.allocated, 0) } }
pub(crate) fn typed_result_unsupported<T>(_value: T, fuel: &Fuel, state: &RuntimeState) -> (u32,u32,u64,u64,u64,u64) { (VELRAN_STATUS_INTERNAL, VELRAN_VALUE_NONE, 0, fuel.used(), state.allocated, 0) }

pub(crate) struct OutputWriter<'a> { buf: &'a mut [u8], required: usize }
impl<'a> OutputWriter<'a> {
    pub(crate) fn new(buf: &'a mut [u8]) -> Self { Self { buf, required: 0 } }
    pub(crate) fn push_bytes(&mut self, bytes: &[u8], state: &mut RuntimeState) -> bool {
        if !state.charge_alloc(bytes.len() as u64) { return false; }
        let start = self.required;
        let Some(required) = self.required.checked_add(bytes.len()) else { state.memory_exceeded(); return false; };
        self.required = required;
        if start < self.buf.len() { let n = bytes.len().min(self.buf.len() - start); self.buf[start..start+n].copy_from_slice(&bytes[..n]); }
        true
    }
    pub(crate) fn push(&mut self, text: &str, state: &mut RuntimeState) -> bool {
        self.push_bytes(text.as_bytes(), state)
    }
    pub(crate) fn push_i64(&mut self, value: i64, state: &mut RuntimeState) -> bool {
        let mut buf = [0u8; 20]; let negative = value < 0; let mut n = value.unsigned_abs(); let mut pos = buf.len();
        loop { pos -= 1; buf[pos] = b'0' + (n % 10) as u8; n /= 10; if n == 0 { break; } }
        if negative { pos -= 1; buf[pos] = b'-'; }
        let text = match std::str::from_utf8(&buf[pos..]) { Ok(v) => v, Err(_) => return false };
        self.push(text, state)
    }
    pub(crate) fn finish(&self, fuel: &Fuel, state: &RuntimeState) -> (u32,u32,u64,u64,u64,u64) {
        self.finish_with_tag(VELRAN_VALUE_HTML, fuel, state)
    }
    pub(crate) fn finish_typed_json(&self, fuel: &Fuel, state: &RuntimeState) -> (u32,u32,u64,u64,u64,u64) {
        self.finish_with_tag(VELRAN_VALUE_TYPED_JSON, fuel, state)
    }
    fn finish_with_tag(&self, tag: u32, fuel: &Fuel, state: &RuntimeState) -> (u32,u32,u64,u64,u64,u64) {
        if self.required > self.buf.len() { (VELRAN_STATUS_OUTPUT_TOO_SMALL, tag, 0, fuel.used(), state.allocated, self.required as u64) }
        else { (VELRAN_STATUS_OK, tag, 0, fuel.used(), state.allocated, self.required as u64) }
    }
}

pub(crate) fn typed_json_begin(out: &mut OutputWriter<'_>, fields: usize, state: &mut RuntimeState) -> bool {
    let Ok(count) = u16::try_from(fields) else { state.bad_request(); return false; };
    out.push_bytes(b"VRJ1", state) && out.push_bytes(&count.to_le_bytes(), state)
}

pub(crate) fn typed_json_field(out: &mut OutputWriter<'_>, name: &str, expected: u8, value: Scalar, state: &mut RuntimeState) -> bool {
    let Ok(name_len) = u16::try_from(name.len()) else { state.bad_request(); return false; };
    if !out.push_bytes(&name_len.to_le_bytes(), state) || !out.push(name, state) || !out.push_bytes(&[expected], state) { return false; }
    match (expected, value) {
        (1, Scalar::Int(v)) => out.push_bytes(&v.to_le_bytes(), state),
        (2, Scalar::Bool(v)) => out.push_bytes(&[u8::from(v)], state),
        (3, Scalar::F32(v)) if v.is_finite() => out.push_bytes(&v.to_bits().to_le_bytes(), state),
        (4, Scalar::String(v)) => {
            let Ok(len) = u32::try_from(v.len()) else { state.memory_exceeded(); return false; };
            out.push_bytes(&len.to_le_bytes(), state) && out.push(&v, state)
        }
        (5, Scalar::StringList(values)) => {
            if values.len() > 4096 { state.memory_exceeded(); return false; }
            let Ok(count) = u32::try_from(values.len()) else { state.memory_exceeded(); return false; };
            if !out.push_bytes(&count.to_le_bytes(), state) { return false; }
            for value in values.iter() {
                let Ok(len) = u32::try_from(value.len()) else { state.memory_exceeded(); return false; };
                if !out.push_bytes(&len.to_le_bytes(), state) || !out.push(&value, state) { return false; }
            }
            true
        }
        _ => { state.bad_request(); false }
    }
}
pub(crate) fn typed_html_i64(value: i64, out: &mut OutputWriter<'_>, state: &mut RuntimeState) -> bool { out.push_i64(value, state) }
pub(crate) fn typed_html_bool(value: bool, out: &mut OutputWriter<'_>, state: &mut RuntimeState) -> bool { out.push(if value { "true" } else { "false" }, state) }
pub(crate) fn typed_html_f32(value: f32, out: &mut OutputWriter<'_>, state: &mut RuntimeState) -> bool { super::velran_numeric::push_f32(value, out, state) }
pub(crate) fn typed_html_unsupported<T>(_value: T, _out: &mut OutputWriter<'_>, state: &mut RuntimeState) -> bool { state.bad_request(); false }
pub(crate) fn html_escape(value: Scalar, out: &mut OutputWriter<'_>, state: &mut RuntimeState) -> Option<()> {
    match value {
        Scalar::Int(v) => if out.push_i64(v, state) { Some(()) } else { None },
        Scalar::Bool(v) => if out.push(if v { "true" } else { "false" }, state) { Some(()) } else { None },
        Scalar::F32(v) => if super::velran_numeric::push_f32(v, out, state) { Some(()) } else { None },
        Scalar::String(v) => if direct_html_string(&v, out, state) { Some(()) } else { None },
        Scalar::F32Array(_) | Scalar::StringList(_) | Scalar::StringDict(_) | Scalar::Struct(_) | Scalar::PureSum(_) | Scalar::Upload(_) | Scalar::Image(_) => None,
    }
}

pub(crate) fn checked_add(a: Scalar, b: Scalar) -> Option<Scalar> { Some(Scalar::Int(as_int(a)?.checked_add(as_int(b)?)?)) }
pub(crate) fn checked_sub(a: Scalar, b: Scalar) -> Option<Scalar> { Some(Scalar::Int(as_int(a)?.checked_sub(as_int(b)?)?)) }
pub(crate) fn checked_mul(a: Scalar, b: Scalar) -> Option<Scalar> { Some(Scalar::Int(as_int(a)?.checked_mul(as_int(b)?)?)) }
pub(crate) fn checked_div(a: Scalar, b: Scalar) -> Option<Scalar> { Some(Scalar::Int(as_int(a)?.checked_div(as_int(b)?)?)) }
pub(crate) fn checked_rem(a: Scalar, b: Scalar) -> Option<Scalar> { Some(Scalar::Int(as_int(a)?.checked_rem(as_int(b)?)?)) }

pub(crate) fn finish_string(text: String) -> Scalar { Scalar::String(Arc::<str>::from(text)) }
pub(crate) fn alloc_string(text: &str, state: &mut RuntimeState) -> Option<Scalar> {
    if !state.charge_alloc((text.len() as u64).saturating_add(24)) { return None; }
    Some(finish_string(text.to_owned()))
}
pub(crate) fn concat_strings(pair: (Option<Scalar>, Option<Scalar>), state: &mut RuntimeState) -> Option<Scalar> {
    let (Some(a), Some(b)) = pair else { return None; };
    let a = as_string(a)?; let b = as_string(b)?;
    let bytes = a.len().saturating_add(b.len());
    if !state.charge_alloc((bytes as u64).saturating_add(24)) { return None; }
    let mut out = String::with_capacity(bytes); out.push_str(&a); out.push_str(&b);
    Some(finish_string(out))
}
pub(crate) fn safe_html_empty() -> Option<Scalar> { Some(Scalar::String(Arc::<str>::from(""))) }
pub(crate) fn safe_html_text(value: Option<Scalar>, state: &mut RuntimeState) -> Option<Scalar> {
    let value = as_string(value?)?;
    let escaped = escape_html_owned(value.as_ref());
    if !state.charge_alloc((escaped.len() as u64).saturating_add(24)) { return None; }
    Some(finish_string(escaped))
}
pub(crate) fn safe_html_concat(left: Option<Scalar>, right: Option<Scalar>, state: &mut RuntimeState) -> Option<Scalar> {
    concat_strings((left, right), state)
}
pub(crate) fn safe_html_element(tag: Option<Scalar>, body: Option<Scalar>, state: &mut RuntimeState) -> Option<Scalar> {
    let tag = as_string(tag?)?;
    let body = as_string(body?)?;
    if !matches!(tag.as_ref(), "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "strong" | "em" | "code" | "pre" | "blockquote" | "ul" | "ol" | "li" | "hr" | "br") {
        state.bad_request(); return None;
    }
    let bytes = tag.len().saturating_mul(2).saturating_add(body.len()).saturating_add(5);
    if !state.charge_alloc((bytes as u64).saturating_add(24)) { return None; }
    let mut out = String::with_capacity(bytes);
    if matches!(tag.as_ref(), "hr" | "br") {
        out.push('<'); out.push_str(tag.as_ref()); out.push('>');
    } else {
        out.push('<'); out.push_str(tag.as_ref()); out.push('>'); out.push_str(body.as_ref()); out.push_str("</"); out.push_str(tag.as_ref()); out.push('>');
    }
    Some(finish_string(out))
}
pub(crate) fn safe_html_link(href: Option<Scalar>, body: Option<Scalar>, state: &mut RuntimeState) -> Option<Scalar> {
    let href = as_string(href?)?;
    let body = as_string(body?)?;
    let lower = href.trim().to_ascii_lowercase();
    let allowed = href.starts_with('/') || href.starts_with('#') || lower.starts_with("https://") || lower.starts_with("http://") || lower.starts_with("mailto:");
    if !allowed || lower.starts_with("javascript:") || lower.starts_with("data:") || lower.starts_with("file:") { state.bad_request(); return None; }
    let attr = escape_html_attr_owned(href.trim());
    let bytes = attr.len().saturating_add(body.len()).saturating_add(15);
    if !state.charge_alloc((bytes as u64).saturating_add(24)) { return None; }
    let mut out = String::with_capacity(bytes);
    out.push_str("<a href=\""); out.push_str(&attr); out.push_str("\">"); out.push_str(body.as_ref()); out.push_str("</a>");
    Some(finish_string(out))
}
fn escape_html_owned(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch { '&' => out.push_str("&amp;"), '<' => out.push_str("&lt;"), '>' => out.push_str("&gt;"), '"' => out.push_str("&quot;"), '\'' => out.push_str("&#39;"), _ => out.push(ch) }
    }
    out
}
fn escape_html_attr_owned(value: &str) -> String { escape_html_owned(value) }
pub(crate) fn string_len(value: Option<Scalar>) -> Option<Scalar> {
    let value = as_string(value?)?;
    let len = if value.is_ascii() { value.len() } else { value.chars().count() };
    Some(Scalar::Int(len.min(i64::MAX as usize) as i64))
}
pub(crate) fn string_trim(value: Option<Scalar>, mode: u8, state: &mut RuntimeState) -> Option<Scalar> {
    let value = as_string(value?)?;
    let trimmed = match mode { 0 => value.trim(), 1 => value.trim_start(), _ => value.trim_end() };
    if trimmed.len() == value.len() { return Some(Scalar::String(value)); }
    if !state.charge_alloc((trimmed.len() as u64).saturating_add(24)) { return None; }
    Some(finish_string(trimmed.to_owned()))
}
pub(crate) fn string_case(value: Option<Scalar>, upper: bool, state: &mut RuntimeState) -> Option<Scalar> {
    let value = as_string(value?)?;
    let estimate = (value.len() as u64).saturating_mul(4).saturating_add(24);
    if !state.charge_alloc(estimate) { return None; }
    Some(finish_string(if upper { value.to_uppercase() } else { value.to_lowercase() }))
}
pub(crate) fn string_predicate<F>(left: Option<Scalar>, right: Option<Scalar>, predicate: F) -> Option<Scalar> where F: FnOnce(&str, &str) -> bool {
    let left = as_string(left?)?; let right = as_string(right?)?; Some(Scalar::Bool(predicate(&left, &right)))
}
pub(crate) fn string_replace(text: Option<Scalar>, from: Option<Scalar>, to: Option<Scalar>, state: &mut RuntimeState) -> Option<Scalar> {
    let text = as_string(text?)?; let from = as_string(from?)?; let to = as_string(to?)?;
    if from.is_empty() { state.bad_request(); return None; }
    if from == to { return Some(Scalar::String(text)); }
    let count = text.matches(from.as_ref()).count() as u64;
    if count == 0 { return Some(Scalar::String(text)); }
    let estimated = (text.len() as u64).saturating_sub(count.saturating_mul(from.len() as u64)).saturating_add(count.saturating_mul(to.len() as u64)).saturating_add(24);
    if !state.charge_alloc(estimated) { return None; }
    Some(finish_string(text.replace(from.as_ref(), to.as_ref())))
}
pub(crate) fn split_bounded(text: Option<Scalar>, delimiter: Option<Scalar>, max_items: Option<Scalar>, state: &mut RuntimeState) -> Option<Scalar> {
    let text = as_string(text?)?; let delimiter = as_string(delimiter?)?; let max_items = usize::try_from(as_int(max_items?)?).ok()?;
    if delimiter.is_empty() || !(1..=4096).contains(&max_items) { state.bad_request(); return None; }
    let piece_count = text.split(delimiter.as_ref()).count();
    if piece_count > max_items { state.bad_request(); return None; }
    let cost = (text.len() as u64).saturating_add((piece_count as u64).saturating_mul(24));
    if !state.charge_alloc(cost) { return None; }
    let items = text.split(delimiter.as_ref()).map(Arc::<str>::from).collect();
    Some(Scalar::StringList(Arc::new(items)))
}
pub(crate) fn substring(text: Option<Scalar>, start: Option<Scalar>, length: Option<Option<Scalar>>, state: &mut RuntimeState) -> Option<Scalar> {
    let text = as_string(text?)?; let start = non_negative_usize(start?, state)?;
    if text.is_ascii() {
        if start > text.len() { state.bad_request(); return None; }
        let available = text.len().saturating_sub(start);
        let take = match length { Some(Some(v)) => non_negative_usize(v, state)?.min(available), Some(None) => return None, None => available };
        let end = start.saturating_add(take);
        let slice = &text[start..end];
        if start == 0 && end == text.len() { return Some(Scalar::String(text)); }
        if !state.charge_alloc((slice.len() as u64).saturating_add(24)) { return None; }
        return Some(finish_string(slice.to_owned()));
    }
    let char_count = text.chars().count();
    if start > char_count { state.bad_request(); return None; }
    let take = match length {
        Some(Some(v)) => non_negative_usize(v, state)?.min(char_count.saturating_sub(start)),
        Some(None) => return None,
        None => char_count.saturating_sub(start),
    };
    if start == 0 && take == char_count { return Some(Scalar::String(text)); }
    if !state.charge_alloc((text.len() as u64).saturating_add(24)) { return None; }
    Some(finish_string(text.chars().skip(start).take(take).collect()))
}
pub(crate) fn index_of(text: Option<Scalar>, needle: Option<Scalar>, reverse: bool) -> Option<Scalar> {
    let text = as_string(text?)?; let needle = as_string(needle?)?;
    let index = if reverse { text.rfind(needle.as_ref()) } else { text.find(needle.as_ref()) };
    let value = index.map(|byte| {
        let chars = if text.is_ascii() { byte } else { text[..byte].chars().count() };
        chars.min(i64::MAX as usize) as i64
    }).unwrap_or(-1);
    Some(Scalar::Int(value))
}
pub(crate) fn char_at(text: Option<Scalar>, index: Option<Scalar>, state: &mut RuntimeState) -> Option<Scalar> {
    let text = as_string(text?)?; let index = non_negative_usize(index?, state)?;
    let ch = if text.is_ascii() {
        let Some(&byte) = text.as_bytes().get(index) else { state.bad_request(); return None; };
        char::from(byte)
    } else {
        let Some(ch) = text.chars().nth(index) else { state.bad_request(); return None; };
        ch
    };
    if !state.charge_alloc(28) { return None; }
    Some(finish_string(ch.to_string()))
}
pub(crate) fn repeat_string(text: Option<Scalar>, count: Option<Scalar>, state: &mut RuntimeState) -> Option<Scalar> {
    let text = as_string(text?)?; let count = non_negative_usize(count?, state)?;
    if count == 1 { return Some(Scalar::String(text)); }
    let Some(bytes) = text.len().checked_mul(count) else { state.bad_request(); return None; };
    if bytes > u64::MAX as usize { state.bad_request(); return None; }
    if !state.charge_alloc((bytes as u64).saturating_add(24)) { return None; }
    Some(finish_string(text.repeat(count)))
}
pub(crate) fn collection_len(value: &Scalar) -> Option<Scalar> {
    let len = match value { Scalar::String(v) => v.chars().count(), Scalar::StringList(v) => v.len(), Scalar::StringDict(v) => v.len(), Scalar::F32Array(v) => v.len(), _ => return None };
    Some(Scalar::Int(i64::try_from(len).ok()?))
}
pub(crate) fn collection_index(value: &Scalar, index: Option<Scalar>, state: &mut RuntimeState) -> Option<Scalar> {
    match value {
        Scalar::StringList(items) => {
            let index = non_negative_usize(index?, state)?;
            items.get(index).map(|v| Scalar::String(Arc::clone(v))).or_else(|| { state.bad_request(); None })
        }
        Scalar::F32Array(items) => {
            let index = non_negative_usize(index?, state)?;
            items.get(index).copied().map(Scalar::F32).or_else(|| { state.bad_request(); None })
        }
        Scalar::StringDict(items) => {
            let key = as_string(index?)?;
            items.get(key.as_ref()).map(|v| Scalar::String(Arc::clone(v))).or_else(|| { state.bad_request(); None })
        }
        _ => None,
    }
}

fn string_dict_clone_cost(items: &BTreeMap<Arc<str>, Arc<str>>) -> u64 {
    items.iter().fold(0u64, |total, (key, value)| {
        total.saturating_add(key.len() as u64).saturating_add(value.len() as u64).saturating_add(64)
    })
}
pub(crate) fn string_dict_new() -> Option<Scalar> {
    Some(Scalar::StringDict(Arc::new(BTreeMap::new())))
}
pub(crate) fn string_dict_contains_key(dict: Option<Scalar>, key: Option<Scalar>) -> Option<Scalar> {
    let Scalar::StringDict(items) = dict? else { return None; };
    let key = as_string(key?)?;
    Some(Scalar::Bool(items.contains_key(key.as_ref())))
}
pub(crate) fn string_dict_set(dict: &mut Scalar, key: Option<Scalar>, value: Option<Scalar>, state: &mut RuntimeState) -> bool {
    let Scalar::StringDict(items) = dict else { state.bad_request(); return false; };
    let Some(key) = key.and_then(as_string) else { state.bad_request(); return false; };
    let Some(value) = value.and_then(as_string) else { state.bad_request(); return false; };
    if items.get(key.as_ref()).is_some_and(|existing| existing.as_ref() == value.as_ref()) { return true; }
    if Arc::strong_count(items) > 1 {
        let clone_cost = string_dict_clone_cost(items);
        if !state.charge_alloc(clone_cost) { return false; }
    }
    let insert_cost = (key.len() as u64).saturating_add(value.len() as u64).saturating_add(64);
    if !state.charge_alloc(insert_cost) { return false; }
    Arc::make_mut(items).insert(key, value);
    true
}
pub(crate) fn string_dict_remove_key(dict: Option<Scalar>, key: Option<Scalar>, state: &mut RuntimeState) -> Option<Scalar> {
    let Some(Scalar::StringDict(mut items)) = dict else { return None; };
    let key = as_string(key?)?;
    if !items.contains_key(key.as_ref()) { return Some(Scalar::StringDict(items)); }
    if Arc::strong_count(&items) > 1 {
        let clone_cost = string_dict_clone_cost(&items);
        if !state.charge_alloc(clone_cost) { return None; }
    }
    Arc::make_mut(&mut items).remove(key.as_ref());
    Some(Scalar::StringDict(items))
}

pub(crate) fn direct_alloc_string(text: &str, state: &mut RuntimeState) -> Option<Arc<str>> {
    if !state.charge_alloc((text.len() as u64).saturating_add(24)) { return None; }
    Some(Arc::<str>::from(text.to_owned()))
}
pub(crate) fn direct_input_int(value: &InputValue<'_>, state: &mut RuntimeState) -> Option<i64> {
    if let InputValue::Int(v) = value { Some(*v) } else { state.bad_request(); None }
}
pub(crate) fn direct_input_bool(value: &InputValue<'_>, state: &mut RuntimeState) -> Option<bool> {
    if let InputValue::Bool(v) = value { Some(*v) } else { state.bad_request(); None }
}
pub(crate) fn direct_input_domain_int(value: &InputValue<'_>, expected: u16, ranges: &[(i64,i64)], state: &mut RuntimeState) -> Option<i64> {
    let InputValue::DomainInt(domain, v) = value else { state.bad_request(); return None; };
    if *domain != expected || !ranges.iter().all(|(min,max)| *v >= *min && *v <= *max) { state.bad_request(); return None; }
    Some(*v)
}
pub(crate) fn direct_input_domain_bool(value: &InputValue<'_>, expected: u16, state: &mut RuntimeState) -> Option<bool> {
    let InputValue::DomainBool(domain, v) = value else { state.bad_request(); return None; };
    if *domain != expected { state.bad_request(); return None; }
    Some(*v)
}
pub(crate) fn direct_input_string(value: &InputValue<'_>, kind: u8, _lengths: &[(usize,usize)], state: &mut RuntimeState) -> Option<Arc<str>> {
    let text = match (kind, value) {
        (0, InputValue::String(v)) | (1, InputValue::Email(v)) | (2, InputValue::Url(v)) | (3, InputValue::Slug(v)) => *v,
        _ => { state.bad_request(); return None; }
    };
    direct_alloc_string(text, state)
}
pub(crate) fn direct_input_domain_string(value: &InputValue<'_>, expected: u16, lengths: &[(usize,usize)], state: &mut RuntimeState) -> Option<Arc<str>> {
    let InputValue::DomainString(domain, v) = value else { state.bad_request(); return None; };
    let chars = if v.is_ascii() { v.len() } else { v.chars().count() };
    if *domain != expected || !lengths.iter().all(|(min,max)| chars >= *min && chars <= *max) { state.bad_request(); return None; }
    direct_alloc_string(v, state)
}
pub(crate) fn direct_html_string(value: &Arc<str>, out: &mut OutputWriter<'_>, state: &mut RuntimeState) -> bool {
    let bytes = value.as_bytes();
    let mut chunk_start = 0usize;
    for (index, byte) in bytes.iter().copied().enumerate() {
        let escaped = match byte {
            b'&' => Some("&amp;"),
            b'<' => Some("&lt;"),
            b'>' => Some("&gt;"),
            b'"' => Some("&quot;"),
            b'\'' => Some("&#39;"),
            _ => None,
        };
        let Some(escaped) = escaped else { continue; };
        if chunk_start < index && !out.push_bytes(&bytes[chunk_start..index], state) { return false; }
        if !out.push(escaped, state) { return false; }
        chunk_start = index + 1;
    }
    chunk_start == bytes.len() || out.push_bytes(&bytes[chunk_start..], state)
}
pub(crate) fn direct_string_len(value: &Arc<str>) -> i64 {
    let len = if value.is_ascii() { value.len() } else { value.chars().count() };
    i64::try_from(len).unwrap_or(i64::MAX)
}
pub(crate) fn direct_concat_strings(a: Arc<str>, b: Arc<str>, state: &mut RuntimeState) -> Option<Arc<str>> {
    let bytes = a.len().checked_add(b.len())?;
    if !state.charge_alloc((bytes as u64).saturating_add(24)) { return None; }
    let mut out=String::with_capacity(bytes); out.push_str(&a); out.push_str(&b); Some(Arc::<str>::from(out))
}
pub(crate) fn direct_safe_html_text(value: Arc<str>, state: &mut RuntimeState) -> Option<Arc<str>> {
    let escaped = escape_html_owned(value.as_ref());
    if !state.charge_alloc((escaped.len() as u64).saturating_add(24)) { return None; }
    Some(Arc::<str>::from(escaped))
}
pub(crate) fn direct_safe_html_element(tag: Arc<str>, body: Arc<str>, state: &mut RuntimeState) -> Option<Arc<str>> {
    let value = safe_html_element(Some(Scalar::String(tag)), Some(Scalar::String(body)), state)?;
    as_string(value)
}
pub(crate) fn direct_safe_html_link(href: Arc<str>, body: Arc<str>, state: &mut RuntimeState) -> Option<Arc<str>> {
    let value = safe_html_link(Some(Scalar::String(href)), Some(Scalar::String(body)), state)?;
    as_string(value)
}
pub(crate) fn direct_string_trim(value: Arc<str>, mode: u8, state: &mut RuntimeState) -> Option<Arc<str>> {
    let trimmed = match mode { 0 => value.trim(), 1 => value.trim_start(), _ => value.trim_end() };
    if trimmed.len() == value.len() { return Some(value); }
    direct_alloc_string(trimmed, state)
}
pub(crate) fn direct_string_case(value: Arc<str>, upper: bool, state: &mut RuntimeState) -> Option<Arc<str>> {
    if value.is_ascii() {
        let changes = if upper { value.as_bytes().iter().any(u8::is_ascii_lowercase) } else { value.as_bytes().iter().any(u8::is_ascii_uppercase) };
        if !changes { return Some(value); }
        let bytes = value.len() as u64;
        if !state.charge_alloc(bytes.saturating_add(24)) { return None; }
        let converted = if upper { value.to_ascii_uppercase() } else { value.to_ascii_lowercase() };
        return Some(Arc::<str>::from(converted));
    }
    let estimate=(value.len() as u64).saturating_mul(4).saturating_add(24);
    if !state.charge_alloc(estimate) { return None; }
    Some(Arc::<str>::from(if upper { value.to_uppercase() } else { value.to_lowercase() }))
}
pub(crate) fn direct_string_replace(text: Arc<str>, from: Arc<str>, to: Arc<str>, state: &mut RuntimeState) -> Option<Arc<str>> {
    if from.is_empty() { state.bad_request(); return None; }
    if from == to { return Some(text); }
    let count=text.matches(from.as_ref()).count() as u64;
    if count==0 { return Some(text); }
    let estimated=(text.len() as u64).saturating_sub(count.saturating_mul(from.len() as u64)).saturating_add(count.saturating_mul(to.len() as u64)).saturating_add(24);
    if !state.charge_alloc(estimated) { return None; }
    Some(Arc::<str>::from(text.replace(from.as_ref(), to.as_ref())))
}
pub(crate) fn direct_split_bounded(text: Arc<str>, delimiter: Arc<str>, max_items: i64, state: &mut RuntimeState) -> Option<Arc<Vec<Arc<str>>>> {
    let max_items=usize::try_from(max_items).ok()?;
    if delimiter.is_empty() || !(1..=4096).contains(&max_items) { state.bad_request(); return None; }
    let piece_count=text.split(delimiter.as_ref()).take(max_items.saturating_add(1)).count();
    if piece_count > max_items { state.bad_request(); return None; }
    let cost=(text.len() as u64).saturating_add((piece_count as u64).saturating_mul(24));
    if !state.charge_alloc(cost) { return None; }
    let mut pieces=Vec::with_capacity(piece_count);
    pieces.extend(text.split(delimiter.as_ref()).map(Arc::<str>::from));
    Some(Arc::new(pieces))
}
pub(crate) fn direct_substring(text: Arc<str>, start: i64, length: Option<i64>, state: &mut RuntimeState) -> Option<Arc<str>> {
    let start=usize::try_from(start).ok().or_else(|| { state.bad_request(); None })?;
    if text.is_ascii() {
        if start > text.len() { state.bad_request(); return None; }
        let available=text.len()-start;
        let take=match length { Some(v) => usize::try_from(v).ok().or_else(|| { state.bad_request(); None })?.min(available), None => available };
        let end=start.checked_add(take)?; let slice=&text[start..end];
        if start==0 && end==text.len() { return Some(text); }
        return direct_alloc_string(slice, state);
    }
    let chars=text.chars().count(); if start > chars { state.bad_request(); return None; }
    let take=match length { Some(v)=>usize::try_from(v).ok().or_else(|| { state.bad_request(); None })?.min(chars-start), None=>chars-start };
    if start==0 && take==chars { return Some(text); }
    let out:String=text.chars().skip(start).take(take).collect();
    if !state.charge_alloc((out.len() as u64).saturating_add(24)) { return None; }
    Some(Arc::<str>::from(out))
}
pub(crate) fn direct_index_of(text: &Arc<str>, needle: &Arc<str>, reverse: bool) -> i64 {
    let index=if reverse { text.rfind(needle.as_ref()) } else { text.find(needle.as_ref()) };
    index.map(|byte| if text.is_ascii() { byte } else { text[..byte].chars().count() }).and_then(|v| i64::try_from(v).ok()).unwrap_or(-1)
}
pub(crate) fn direct_char_at(text: Arc<str>, index: i64, state: &mut RuntimeState) -> Option<Arc<str>> {
    let index=usize::try_from(index).ok().or_else(|| { state.bad_request(); None })?;
    let ch=if text.is_ascii() { char::from(*text.as_bytes().get(index).or_else(|| { state.bad_request(); None })?) }
        else { text.chars().nth(index).or_else(|| { state.bad_request(); None })? };
    let mut tmp=[0u8;4]; direct_alloc_string(ch.encode_utf8(&mut tmp), state)
}
pub(crate) fn direct_repeat_string(text: Arc<str>, count: i64, state: &mut RuntimeState) -> Option<Arc<str>> {
    let count=usize::try_from(count).ok().or_else(|| { state.bad_request(); None })?;
    if count==1 { return Some(text); }
    let bytes=text.len().checked_mul(count).or_else(|| { state.bad_request(); None })?;
    if !state.charge_alloc((bytes as u64).saturating_add(24)) { return None; }
    Some(Arc::<str>::from(text.repeat(count)))
}
pub(crate) fn direct_list_index(items: &[Arc<str>], index: i64, state: &mut RuntimeState) -> Option<Arc<str>> {
    let index=usize::try_from(index).ok().or_else(|| { state.bad_request(); None })?;
    items.get(index).cloned().or_else(|| { state.bad_request(); None })
}
pub(crate) fn direct_dict_index(items: &Arc<BTreeMap<Arc<str>,Arc<str>>>, key: &Arc<str>, state: &mut RuntimeState) -> Option<Arc<str>> {
    items.get(key.as_ref()).cloned().or_else(|| { state.bad_request(); None })
}
pub(crate) fn direct_dict_set(items: &mut Arc<BTreeMap<Arc<str>,Arc<str>>>, key: Arc<str>, value: Arc<str>, state: &mut RuntimeState) -> bool {
    if items.get(key.as_ref()).is_some_and(|existing| existing.as_ref()==value.as_ref()) { return true; }
    if Arc::strong_count(items)>1 && !state.charge_alloc(string_dict_clone_cost(items)) { return false; }
    let cost=(key.len() as u64).saturating_add(value.len() as u64).saturating_add(64);
    if !state.charge_alloc(cost) { return false; }
    Arc::make_mut(items).insert(key,value); true
}
pub(crate) fn direct_dict_remove_key(mut items: Arc<BTreeMap<Arc<str>,Arc<str>>>, key: Arc<str>, state: &mut RuntimeState) -> Option<Arc<BTreeMap<Arc<str>,Arc<str>>>> {
    if !items.contains_key(key.as_ref()) { return Some(items); }
    if Arc::strong_count(&items)>1 && !state.charge_alloc(string_dict_clone_cost(&items)) { return None; }
    Arc::make_mut(&mut items).remove(key.as_ref()); Some(items)
}

}
use velran_support::*;

"#;

#[cfg(test)]
mod scope_regression_tests {
    use super::*;

    #[test]
    fn support_module_imports_host_abi_in_its_own_scope() {
        assert!(SUPPORT_SOURCE.contains("use crate::{VelranHostApi, VelranHostResult};"));
        assert!(SUPPORT_SOURCE.contains("VELRAN_MODULE_ABI_VERSION"));
        assert!(SUPPORT_SOURCE.contains("use std::ffi::c_void;"));
    }

    #[test]
    fn support_module_uses_sibling_numeric_module() {
        assert!(SUPPORT_SOURCE.contains("super::velran_numeric::push_f32"));
        assert!(!SUPPORT_SOURCE.contains("crate::velran_numeric::push_f32"));
    }

    #[test]
    fn direct_html_escape_is_chunked_and_bounded() {
        assert!(SUPPORT_SOURCE.contains("let mut chunk_start = 0usize;"));
        assert!(SUPPORT_SOURCE.contains("out.push_bytes(&bytes[chunk_start..index], state)"));
        assert!(SUPPORT_SOURCE.contains("b'&' => Some(\"&amp;\")"));
        assert!(!SUPPORT_SOURCE.contains("for ch in value.chars() {\n        let ok = match ch"));
    }

    #[test]
    fn bounded_split_stops_counting_after_the_declared_limit() {
        assert!(SUPPORT_SOURCE.contains("take(max_items.saturating_add(1)).count()"));
        assert!(SUPPORT_SOURCE.contains("Vec::with_capacity(piece_count)"));
        assert!(SUPPORT_SOURCE.contains("Some(Arc::new(pieces))"));
    }
}
