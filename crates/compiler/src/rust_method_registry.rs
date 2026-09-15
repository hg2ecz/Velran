use language_core::{BuiltinFunction, PureSumPredicate};

/// Semantic meaning of a Rust-surface method after name resolution.
///
/// The parser does not maintain its own method allow-list. It parses the
/// generic `receiver.method(args...)` shape and asks this registry to resolve
/// the method into a compiler-owned semantic operation. Receiver type and
/// security validation happen later in the type/security verifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RustMethod {
    Builtin(BuiltinFunction),
    CollectionLen,
    Elapsed,
    CharsCount,
    SumPredicate(PureSumPredicate),
    SumUnwrapOr,
    StringToOwned,
}

/// Resolve only authority-free Rust-surface methods.
///
/// Ambient-authority APIs are intentionally absent. Filesystem, network,
/// process, environment, thread, FFI and similar effects must enter through
/// explicit framework capability/effect boundaries rather than method-name
/// passthrough.
pub(super) fn resolve(name: &str) -> Option<RustMethod> {
    use RustMethod::*;
    let method = match name {
        "len" => CollectionLen,
        "elapsed" => Elapsed,
        "chars" => CharsCount,
        "is_some" => SumPredicate(PureSumPredicate::IsSome),
        "is_none" => SumPredicate(PureSumPredicate::IsNone),
        "is_ok" => SumPredicate(PureSumPredicate::IsOk),
        "is_err" => SumPredicate(PureSumPredicate::IsErr),
        "unwrap_or" => SumUnwrapOr,
        "to_string" => StringToOwned,
        "trim" => Builtin(BuiltinFunction::Trim),
        "trim_start" => Builtin(BuiltinFunction::TrimStart),
        "trim_end" => Builtin(BuiltinFunction::TrimEnd),
        "to_lowercase" => Builtin(BuiltinFunction::Lower),
        "to_uppercase" => Builtin(BuiltinFunction::Upper),
        "contains" => Builtin(BuiltinFunction::Contains),
        "starts_with" => Builtin(BuiltinFunction::StartsWith),
        "ends_with" => Builtin(BuiltinFunction::EndsWith),
        "replace" => Builtin(BuiltinFunction::Replace),
        "repeat" => Builtin(BuiltinFunction::Repeat),
        "contains_key" => Builtin(BuiltinFunction::ContainsKey),
        "sin" => Builtin(BuiltinFunction::Sin),
        "cos" => Builtin(BuiltinFunction::Cos),
        "sqrt" => Builtin(BuiltinFunction::Sqrt),
        "abs" => Builtin(BuiltinFunction::Abs),
        "ln" => Builtin(BuiltinFunction::Ln),
        "log10" => Builtin(BuiltinFunction::Log10),
        "log" => Builtin(BuiltinFunction::Log),
        "exp" => Builtin(BuiltinFunction::Exp),
        "powf" => Builtin(BuiltinFunction::Pow),
        "round" => Builtin(BuiltinFunction::Round),
        "floor" => Builtin(BuiltinFunction::Floor),
        "ceil" => Builtin(BuiltinFunction::Ceil),
        _ => return None,
    };
    Some(method)
}
