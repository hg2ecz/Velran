use crate::expression_parser::ExprParser;
use crate::expression_token::ExprToken;
use crate::{CompileError, rust_method_registry};
use language_core::{BinaryOp, BuiltinFunction, Expr};
use rust_method_registry::RustMethod;

impl ExprParser<'_> {
    pub(super) fn parse_postfix(&mut self, mut expr: Expr) -> Result<Expr, CompileError> {
        loop {
            if self.tokens.get(self.pos) != Some(&ExprToken::Dot) {
                return Ok(expr);
            }
            self.pos += 1;
            let member = match self.tokens.get(self.pos).cloned() {
                Some(ExprToken::Ident(name)) => name,
                _ => {
                    return Err(CompileError::Syntax(
                        "field or method name expected after `.`".into(),
                    ));
                }
            };
            self.pos += 1;

            // No call syntax means field access. Current verified field access
            // intentionally remains rooted at a named model/upload local.
            if self.tokens.get(self.pos) != Some(&ExprToken::LParen) {
                expr = match expr {
                    Expr::Variable(base) => Expr::Field {
                        base,
                        field: member,
                    },
                    _ => {
                        return Err(CompileError::Syntax(
                            "chained field access is not supported by the verified field model"
                                .into(),
                        ));
                    }
                };
                continue;
            }

            let method = rust_method_registry::resolve(&member).ok_or_else(|| {
                CompileError::Syntax(format!(
                    "unsupported or capability-requiring method `.{member}(...)`"
                ))
            })?;

            match method {
                RustMethod::SumPredicate(predicate) => {
                    self.expect_empty_call(&member)?;
                    let Expr::Variable(base) = expr else {
                        return Err(CompileError::Syntax(format!(
                            ".{member}() currently requires a named Option/Result local"
                        )));
                    };
                    expr = Expr::PureSumPredicate { base, predicate };
                }
                RustMethod::SumUnwrapOr => {
                    let Expr::Variable(base) = expr else {
                        return Err(CompileError::Syntax(
                            ".unwrap_or(...) currently requires a named Option/Result local".into(),
                        ));
                    };
                    self.pos += 1; // `(`
                    let fallback = self.parse_logical_or()?;
                    if self.tokens.get(self.pos) != Some(&ExprToken::RParen) {
                        return Err(CompileError::Syntax(
                            ".unwrap_or(...) requires exactly one fallback expression".into(),
                        ));
                    }
                    self.pos += 1;
                    expr = Expr::PureSumUnwrapOr {
                        base,
                        fallback: Box::new(fallback),
                    };
                }
                RustMethod::CollectionLen => {
                    self.expect_empty_call("len")?;
                    expr = match expr {
                        Expr::Variable(collection) => Expr::CollectionLen { collection },
                        _ => {
                            return Err(CompileError::Syntax(
                                ".len() currently requires a named local collection".into(),
                            ));
                        }
                    };
                }
                RustMethod::Elapsed => {
                    self.expect_empty_call("elapsed")?;
                    if self.tokens.get(self.pos) != Some(&ExprToken::Dot)
                        || self.tokens.get(self.pos + 1)
                            != Some(&ExprToken::Ident("as_nanos".into()))
                    {
                        return Err(CompileError::Syntax(
                            "Velran exposes Rust-like elapsed timing as `started.elapsed().as_nanos()`".into(),
                        ));
                    }
                    self.pos += 2;
                    self.expect_empty_call("as_nanos")?;
                    expr = Expr::Binary {
                        left: Box::new(Expr::Builtin {
                            function: BuiltinFunction::MonotonicNanos,
                            args: Vec::new(),
                        }),
                        op: BinaryOp::Sub,
                        right: Box::new(expr),
                    };
                }
                RustMethod::CharsCount => {
                    self.expect_empty_call("chars")?;
                    if self.tokens.get(self.pos) != Some(&ExprToken::Dot) {
                        return Err(CompileError::Syntax(
                            "Velran currently exposes Rust-like character counting as `.chars().count()`".into(),
                        ));
                    }
                    self.pos += 1;
                    if self.tokens.get(self.pos) != Some(&ExprToken::Ident("count".into())) {
                        return Err(CompileError::Syntax(
                            "Velran currently exposes Rust-like character counting as `.chars().count()`".into(),
                        ));
                    }
                    self.pos += 1;
                    self.expect_empty_call("count")?;
                    expr = Expr::Builtin {
                        function: BuiltinFunction::StringLen,
                        args: vec![expr],
                    };
                }
                RustMethod::Builtin(function) => {
                    self.pos += 1; // `(`
                    let mut args = vec![expr];
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
                        return Err(CompileError::Syntax(format!(".{member}(...) missing `)`")));
                    }
                    self.pos += 1;
                    expr = Expr::Builtin { function, args };
                }
            }
        }
    }

    fn expect_empty_call(&mut self, method: &str) -> Result<(), CompileError> {
        if self.tokens.get(self.pos) != Some(&ExprToken::LParen)
            || self.tokens.get(self.pos + 1) != Some(&ExprToken::RParen)
        {
            return Err(CompileError::Syntax(format!(
                ".{method}() does not take arguments"
            )));
        }
        self.pos += 2;
        Ok(())
    }
}
