use crate::expression_parser::ExprParser;
use crate::expression_token::ExprToken;
use crate::module_namespace::resolve;
use crate::{CompileError, builtin_registry};
use language_core::{BuiltinFunction, Expr};

impl ExprParser<'_> {
    pub(super) fn parse_primary(&mut self) -> Result<Expr, CompileError> {
        let tok = self
            .tokens
            .get(self.pos)
            .ok_or_else(|| CompileError::Syntax("expected expression".into()))?
            .clone();
        self.pos += 1;
        match tok {
            ExprToken::String(v) => Ok(Expr::String(v)),
            ExprToken::Int(v) => Ok(Expr::Int(v)),
            ExprToken::F32(v) => Ok(Expr::F32(v)),
            ExprToken::Minus => {
                let inner = self.parse_primary()?;
                match inner {
                    Expr::Int(v) => v
                        .checked_neg()
                        .map(Expr::Int)
                        .ok_or_else(|| CompileError::Syntax("integer out of range".into())),
                    Expr::F32(v) => language_core::F32Value::new(-v.get())
                        .map(Expr::F32)
                        .ok_or_else(|| {
                            CompileError::Syntax("F32 literal must be finite and in range".into())
                        }),
                    _ => Err(CompileError::Syntax(
                        "unary - requires Int or F32 literal".into(),
                    )),
                }
            }
            ExprToken::Ident(v) if v == "true" => Ok(Expr::Bool(true)),
            ExprToken::Ident(v) if v == "false" => Ok(Expr::Bool(false)),
            ExprToken::Ident(v) => {
                if v == "vec"
                    && self.tokens.get(self.pos) == Some(&ExprToken::Bang)
                    && self.tokens.get(self.pos + 1) == Some(&ExprToken::LBracket)
                {
                    self.pos += 2;
                    let fill = self.parse_logical_or()?;
                    if self.tokens.get(self.pos) != Some(&ExprToken::Semi) {
                        return Err(CompileError::Syntax(
                            "vec![fill; len] expects `;` between fill and length".into(),
                        ));
                    }
                    self.pos += 1;
                    let len = self.parse_logical_or()?;
                    if self.tokens.get(self.pos) != Some(&ExprToken::RBracket) {
                        return Err(CompileError::Syntax(
                            "vec![fill; len] missing closing ]".into(),
                        ));
                    }
                    self.pos += 1;
                    return Ok(Expr::F32ArrayNew {
                        len: Box::new(len),
                        fill: Box::new(fill),
                    });
                }
                if let Some((type_path, variant)) = v.rsplit_once("::") {
                    let enum_name = resolve(self.namespace, type_path);
                    if let Some((enum_id, def)) = self.program.enum_by_name(&enum_name) {
                        if !def.variants.iter().any(|x| x == variant) {
                            return Err(CompileError::Syntax(format!(
                                "enum `{enum_name}` has no variant `{variant}`"
                            )));
                        }
                        return Ok(Expr::EnumLiteral {
                            enum_id,
                            variant: variant.into(),
                        });
                    }
                }
                if v == "toF32" && self.tokens.get(self.pos) == Some(&ExprToken::LParen) {
                    return Err(CompileError::Syntax(
                        "legacy `toF32(...)` syntax is not supported; use Rust-like `value as f32`"
                            .into(),
                    ));
                }
                if v == "std::time::Instant::now"
                    && self.tokens.get(self.pos) == Some(&ExprToken::LParen)
                {
                    self.pos += 1;
                    if self.tokens.get(self.pos) != Some(&ExprToken::RParen) {
                        return Err(CompileError::Syntax(
                            "std::time::Instant::now() does not take arguments".into(),
                        ));
                    }
                    self.pos += 1;
                    return Ok(Expr::Builtin {
                        function: BuiltinFunction::MonotonicNanos,
                        args: Vec::new(),
                    });
                }
                if v == "monotonicNanos" && self.tokens.get(self.pos) == Some(&ExprToken::LParen) {
                    return Err(CompileError::Syntax(
                        "legacy `monotonicNanos()` syntax is not supported; use Rust-like `std::time::Instant::now()` and `started.elapsed().as_nanos()`".into(),
                    ));
                }
                if matches!(
                    v.as_str(),
                    "sin"
                        | "cos"
                        | "sqrt"
                        | "abs"
                        | "ln"
                        | "log10"
                        | "log"
                        | "exp"
                        | "pow"
                        | "round"
                        | "floor"
                        | "ceil"
                ) && self.tokens.get(self.pos) == Some(&ExprToken::LParen)
                {
                    let replacement = match v.as_str() {
                        "sin" => "value.sin()",
                        "cos" => "value.cos()",
                        "sqrt" => "value.sqrt()",
                        "abs" => "value.abs()",
                        "ln" => "value.ln()",
                        "log10" => "value.log10()",
                        "log" => "value.log(base)",
                        "exp" => "value.exp()",
                        "pow" => "value.powf(exp)",
                        "round" => "value.round()",
                        "floor" => "value.floor()",
                        "ceil" => "value.ceil()",
                        _ => unreachable!(),
                    };
                    return Err(CompileError::Syntax(format!(
                        "legacy `{v}(...)` syntax is not supported; use Rust-like `{replacement}`"
                    )));
                }
                if v == "split" && self.tokens.get(self.pos) == Some(&ExprToken::LParen) {
                    return Err(CompileError::Syntax(
                        "unbounded `split(...)` is not supported in native-only Velran; use `splitBounded(text, delimiter, maxItems)` with an explicit compile-time bound".into(),
                    ));
                }
                if matches!(
                    v.as_str(),
                    "trim"
                        | "trimStart"
                        | "trimEnd"
                        | "lower"
                        | "upper"
                        | "contains"
                        | "startsWith"
                        | "endsWith"
                        | "replace"
                        | "repeat"
                        | "stringLen"
                        | "dict"
                        | "containsKey"
                ) && self.tokens.get(self.pos) == Some(&ExprToken::LParen)
                {
                    let replacement = match v.as_str() {
                        "trim" => "value.trim()",
                        "trimStart" => "value.trim_start()",
                        "trimEnd" => "value.trim_end()",
                        "lower" => "value.to_lowercase()",
                        "upper" => "value.to_uppercase()",
                        "contains" => "value.contains(needle)",
                        "startsWith" => "value.starts_with(prefix)",
                        "endsWith" => "value.ends_with(suffix)",
                        "replace" => "value.replace(from, to)",
                        "repeat" => "value.repeat(count)",
                        "stringLen" => "value.chars().count()",
                        "dict" => "BTreeMap::new()",
                        "containsKey" => "map.contains_key(key)",
                        _ => unreachable!(),
                    };
                    return Err(CompileError::Syntax(format!(
                        "legacy `{v}(...)` syntax is not supported; use Rust-like `{replacement}`"
                    )));
                }
                if v == "BTreeMap::new" && self.tokens.get(self.pos) == Some(&ExprToken::LParen) {
                    self.pos += 1;
                    if self.tokens.get(self.pos) != Some(&ExprToken::RParen) {
                        return Err(CompileError::Syntax(
                            "BTreeMap::new() does not take arguments".into(),
                        ));
                    }
                    self.pos += 1;
                    return Ok(Expr::Builtin {
                        function: BuiltinFunction::DictNew,
                        args: Vec::new(),
                    });
                }
                if let Some(function) = builtin_registry::resolve(&v)
                    && self.tokens.get(self.pos) == Some(&ExprToken::LParen)
                {
                    self.pos += 1;
                    let mut args = Vec::new();
                    if self.tokens.get(self.pos) != Some(&ExprToken::RParen) {
                        loop {
                            args.push(self.parse_logical_or()?);
                            if self.tokens.get(self.pos) == Some(&ExprToken::Comma) {
                                self.pos += 1;
                                continue;
                            }
                            break;
                        }
                    }
                    if self.tokens.get(self.pos) != Some(&ExprToken::RParen) {
                        return Err(CompileError::Syntax(format!("{v}(...) missing )")));
                    }
                    self.pos += 1;
                    Ok(Expr::Builtin { function, args })
                } else if v == "slug" && self.tokens.get(self.pos) == Some(&ExprToken::LParen) {
                    self.pos += 1;
                    let inner = self.parse_logical_or()?;
                    if self.tokens.get(self.pos) != Some(&ExprToken::RParen) {
                        return Err(CompileError::Syntax(
                            "slug(...) expects exactly one expression".into(),
                        ));
                    }
                    self.pos += 1;
                    Ok(Expr::Slugify(Box::new(inner)))
                } else if matches!(v.as_str(), "arrayF32" | "len")
                    && self.tokens.get(self.pos) == Some(&ExprToken::LParen)
                {
                    return Err(CompileError::Syntax(format!(
                        "legacy `{v}(...)` syntax is not supported; use Rust-like `vec![fill; len]` or `.len()`"
                    )));
                } else if self.tokens.get(self.pos) == Some(&ExprToken::LBracket) {
                    self.pos += 1;
                    let index = self.parse_logical_or()?;
                    if self.tokens.get(self.pos) != Some(&ExprToken::RBracket) {
                        return Err(CompileError::Syntax("array index missing ]".into()));
                    }
                    self.pos += 1;
                    Ok(Expr::CollectionIndex {
                        collection: v,
                        index: Box::new(index),
                    })
                } else {
                    Ok(Expr::Variable(v))
                }
            }
            ExprToken::LParen => {
                let e = self.parse_logical_or()?;
                if self.tokens.get(self.pos) != Some(&ExprToken::RParen) {
                    return Err(CompileError::Syntax("expected )".into()));
                }
                self.pos += 1;
                Ok(e)
            }
            _ => Err(CompileError::Syntax("expected expression value".into())),
        }
    }
}
