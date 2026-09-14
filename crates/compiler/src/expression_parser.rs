use crate::expression_token::ExprToken;
use crate::{CompileError, lexer};
use language_core::{BinaryOp, Expr, Program};

pub(super) fn parse_expr_in_namespace(
    input: &str,
    namespace: &str,
    program: &Program,
) -> Result<Expr, CompileError> {
    let tokens = lexer::lex_expr(input)?;
    let mut p = ExprParser {
        tokens: &tokens,
        pos: 0,
        namespace,
        program,
    };
    let e = p.parse_logical_or()?;
    if p.pos != tokens.len() {
        return Err(CompileError::Syntax(format!(
            "unexpected token in expression `{input}`"
        )));
    }
    Ok(e)
}

fn binary(left: Expr, op: BinaryOp, right: Expr) -> Expr {
    Expr::Binary {
        left: Box::new(left),
        op,
        right: Box::new(right),
    }
}

pub(super) struct ExprParser<'a> {
    pub(super) tokens: &'a [ExprToken],
    pub(super) pos: usize,
    pub(super) namespace: &'a str,
    pub(super) program: &'a Program,
}
impl ExprParser<'_> {
    pub(super) fn parse_logical_or(&mut self) -> Result<Expr, CompileError> {
        self.parse_left_associative(
            Self::parse_logical_and,
            &[(ExprToken::OrOr, BinaryOp::LogicalOr)],
        )
    }

    fn parse_logical_and(&mut self) -> Result<Expr, CompileError> {
        self.parse_left_associative(
            Self::parse_bit_or,
            &[(ExprToken::AndAnd, BinaryOp::LogicalAnd)],
        )
    }

    fn parse_bit_or(&mut self) -> Result<Expr, CompileError> {
        self.parse_left_associative(Self::parse_bit_xor, &[(ExprToken::Pipe, BinaryOp::BitOr)])
    }

    fn parse_bit_xor(&mut self) -> Result<Expr, CompileError> {
        self.parse_left_associative(Self::parse_bit_and, &[(ExprToken::Caret, BinaryOp::BitXor)])
    }

    fn parse_bit_and(&mut self) -> Result<Expr, CompileError> {
        self.parse_left_associative(Self::parse_compare, &[(ExprToken::Amp, BinaryOp::BitAnd)])
    }

    fn parse_compare(&mut self) -> Result<Expr, CompileError> {
        let mut left = self.parse_shift()?;
        let op = match self.tokens.get(self.pos) {
            Some(ExprToken::Lt) => Some(BinaryOp::Lt),
            Some(ExprToken::Le) => Some(BinaryOp::Le),
            Some(ExprToken::Gt) => Some(BinaryOp::Gt),
            Some(ExprToken::Ge) => Some(BinaryOp::Ge),
            Some(ExprToken::EqEq) => Some(BinaryOp::Eq),
            Some(ExprToken::Ne) => Some(BinaryOp::Ne),
            _ => None,
        };
        if let Some(op) = op {
            self.pos += 1;
            let right = self.parse_shift()?;
            left = binary(left, op, right);
            if matches!(
                self.tokens.get(self.pos),
                Some(
                    ExprToken::Lt
                        | ExprToken::Le
                        | ExprToken::Gt
                        | ExprToken::Ge
                        | ExprToken::EqEq
                        | ExprToken::Ne
                )
            ) {
                return Err(CompileError::Syntax(
                    "chained comparisons are not supported".into(),
                ));
            }
        }
        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<Expr, CompileError> {
        self.parse_left_associative(
            Self::parse_add_sub,
            &[
                (ExprToken::ShiftLeft, BinaryOp::ShiftLeft),
                (ExprToken::ShiftRight, BinaryOp::ShiftRight),
            ],
        )
    }

    fn parse_add_sub(&mut self) -> Result<Expr, CompileError> {
        self.parse_left_associative(
            Self::parse_mul_div_rem,
            &[
                (ExprToken::Plus, BinaryOp::Add),
                (ExprToken::Minus, BinaryOp::Sub),
            ],
        )
    }

    fn parse_mul_div_rem(&mut self) -> Result<Expr, CompileError> {
        self.parse_left_associative(
            Self::parse_cast,
            &[
                (ExprToken::Star, BinaryOp::Mul),
                (ExprToken::Slash, BinaryOp::Div),
                (ExprToken::Percent, BinaryOp::Rem),
            ],
        )
    }

    fn parse_cast(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.parse_unary()?;
        while matches!(self.tokens.get(self.pos), Some(ExprToken::Ident(v)) if v == "as") {
            self.pos += 1;
            let target = match self.tokens.get(self.pos).cloned() {
                Some(ExprToken::Ident(v)) => v,
                _ => return Err(CompileError::Syntax("type expected after `as`".into())),
            };
            self.pos += 1;
            expr = match target.as_str() {
                "f32" => Expr::Builtin {
                    function: language_core::BuiltinFunction::ToF32,
                    args: vec![expr],
                },
                _ => {
                    return Err(CompileError::Syntax(format!(
                        "unsupported cast target `{target}`; currently only Rust-like `as f32` is supported"
                    )));
                }
            };
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, CompileError> {
        if self.tokens.get(self.pos) == Some(&ExprToken::Bang) {
            self.pos += 1;
            return Ok(Expr::Not(Box::new(self.parse_unary()?)));
        }
        let primary = self.parse_primary()?;
        self.parse_postfix(primary)
    }

    fn parse_left_associative(
        &mut self,
        next: fn(&mut Self) -> Result<Expr, CompileError>,
        operators: &[(ExprToken, BinaryOp)],
    ) -> Result<Expr, CompileError> {
        let mut left = next(self)?;
        loop {
            let Some((_, op)) = operators
                .iter()
                .find(|(token, _)| self.tokens.get(self.pos) == Some(token))
            else {
                break;
            };
            self.pos += 1;
            let right = next(self)?;
            left = binary(left, *op, right);
        }
        Ok(left)
    }
}
