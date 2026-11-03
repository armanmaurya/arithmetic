//! The parser: turns tokens into a tree (`Expr`). It decides structure only;
//! all maths happens in the evaluator.
//!
//! Grammar, lowest precedence first. Each rule is one function below, and
//! each level is built from the one after it, which is what makes
//! `×` bind tighter than `+`:
//!
//! ```text
//! program   = statement ( ":" statement )*
//! statement = value ( "→" VARIABLE )?
//! value     = expr ( "∠" expr )?                       ← polar complex: 2∠30
//! expr      = term    ( ("+" | "−") term )*
//! term      = combin  ( ("×" | "÷") combin | unary )*   ← 2nd form: implicit ×, e.g. 2π
//! combin    = unary   ( ("nPr" | "nCr") unary )*        ← 7nCr3, as typed on a calculator
//! unary     = ("−" | "+") unary | power
//! power     = postfix ( ("^" | "ˣ√") unary )?          ← right side recurses: 2^3^2 = 2^(3^2)
//! postfix   = primary ( "!" | "%" | dms | "ʳ" | "ᵍ" )*
//! dms       = "°" ( NUMBER "′" )? ( NUMBER "″" )?      ← 12°30′15″
//! primary   = NUMBER | CONSTANT | VARIABLE | UNIT | matrix | list | call | "(" value ")"
//! call      = FUNC "(" ( argument ( "," argument )* )? ")"
//!           | FUNC bare                                ← √9, sin 30°, log 10^3
//! bare      = "−" bare | power                         ← sin −30
//!           | FUNC                                     ← functions without arguments: rand
//! argument  = value ( "=" value )?                     ← equations, for solve()
//! matrix    = "[" row ( ";" row )* "]"
//! row       = expr ( "," expr )*
//! list      = "{" ( value ( "," value )* )? "}"
//! ```
//!
//! With [`ParseOptions::implicit_mul_first`], implicit multiplication gets
//! its own level between `combin` and `unary`, so `6÷2(1+2)` is `6÷(2×3)`.
//!
//! `nPr` and `nCr` are infix when no `(` follows them (`7nCr3`), and
//! ordinary functions otherwise (`nCr(7,3)`). `xˣ√y` is `root(y, x)`.
//!
//! A missing `)`, `]` or `}` at the very end is allowed, so `sin(30` works
//! while typing.

mod ast;
mod error;

#[cfg(test)]
mod tests;

pub use ast::{BinaryOp, Expr, PostfixOp, UnaryOp};
pub use error::ParseError;

use crate::core::engine::{
    functions::FuncId,
    lexer::{Span, Token, TokenKind, Var},
    settings::AngleMode,
};

/// Deepest nesting of brackets and signs. Keeps absurd input like 10 000
/// `(` from overflowing the stack.
const MAX_DEPTH: usize = 100;

/// Grammar choices on which calculators disagree.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ParseOptions {
    /// Implicit multiplication binds tighter than `×` and `÷`.
    pub implicit_mul_first: bool,
}

pub fn parse(tokens: &[Token]) -> Result<Expr, ParseError> {
    parse_with(tokens, ParseOptions::default())
}

pub fn parse_with(tokens: &[Token], options: ParseOptions) -> Result<Expr, ParseError> {
    let tokens = &carry_on_from_ans(tokens);
    let mut parser = Parser {
        tokens,
        pos: 0,
        depth: 0,
        options,
    };
    let expr = parser.program()?;
    match parser.peek() {
        None => Ok(expr),
        Some(TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace) => {
            Err(ParseError::UnmatchedBracket {
                span: parser.span(),
            })
        }
        Some(_) => Err(ParseError::UnexpectedToken {
            span: parser.span(),
        }),
    }
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    depth: usize,
    options: ParseOptions,
}

impl Parser<'_> {
    fn peek(&self) -> Option<TokenKind> {
        self.peek_at(0)
    }

    fn peek_at(&self, ahead: usize) -> Option<TokenKind> {
        self.tokens.get(self.pos + ahead).map(|t| t.kind)
    }

    fn advance(&mut self) -> Option<TokenKind> {
        let kind = self.peek();
        if kind.is_some() {
            self.pos += 1;
        }
        kind
    }

    /// Consumes the next token if it is `kind`.
    fn eat(&mut self, kind: TokenKind) -> bool {
        if self.peek() == Some(kind) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// The span of the current token, or an empty span at the end of input.
    fn span(&self) -> Span {
        match self.tokens.get(self.pos) {
            Some(t) => t.span.clone(),
            None => {
                let end = self.tokens.last().map_or(0, |t| t.span.end);
                end..end
            }
        }
    }

    /// The end of the last consumed token.
    fn last_end(&self) -> usize {
        self.pos
            .checked_sub(1)
            .and_then(|i| self.tokens.get(i))
            .map_or(0, |t| t.span.end)
    }

    /// The error for "something else was expected here".
    fn unexpected(&self) -> ParseError {
        if self.pos < self.tokens.len() {
            ParseError::UnexpectedToken { span: self.span() }
        } else {
            ParseError::UnexpectedEnd { span: self.span() }
        }
    }

    /// Consumes a closing bracket. At the very end of the input it may be
    /// missing; anything else in its place is an error.
    fn close(&mut self, kind: TokenKind) -> Result<(), ParseError> {
        if self.eat(kind) || self.peek().is_none() {
            Ok(())
        } else {
            Err(self.unexpected())
        }
    }

    /// Runs `f` one nesting level deeper.
    fn nested<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, ParseError>,
    ) -> Result<T, ParseError> {
        if self.depth >= MAX_DEPTH {
            return Err(ParseError::TooDeep { span: self.span() });
        }
        self.depth += 1;
        let result = f(self);
        self.depth -= 1;
        result
    }

    fn program(&mut self) -> Result<Expr, ParseError> {
        let first = self.statement()?;
        if self.peek() != Some(TokenKind::Colon) {
            return Ok(first);
        }
        let mut items = vec![first];
        while self.eat(TokenKind::Colon) {
            items.push(self.statement()?);
        }
        Ok(Expr::Sequence(items))
    }

    fn statement(&mut self) -> Result<Expr, ParseError> {
        let value = self.value()?;
        if !self.eat(TokenKind::Store) {
            return Ok(value);
        }
        match self.peek() {
            Some(TokenKind::Variable(target)) if target != Var::Ans => {
                self.advance();
                Ok(Expr::Assign {
                    target,
                    value: Box::new(value),
                })
            }
            _ => Err(self.unexpected()),
        }
    }

    fn value(&mut self) -> Result<Expr, ParseError> {
        let left = self.expr()?;
        if self.eat(TokenKind::Angle) {
            return Ok(Expr::binary(BinaryOp::Polar, left, self.expr()?));
        }
        Ok(left)
    }

    fn expr(&mut self) -> Result<Expr, ParseError> {
        self.nested(|p| {
            let mut left = p.term()?;
            loop {
                let op = match p.peek() {
                    Some(TokenKind::Plus) => BinaryOp::Add,
                    Some(TokenKind::Minus) => BinaryOp::Sub,
                    _ => return Ok(left),
                };
                p.advance();
                left = Expr::binary(op, left, p.term()?);
            }
        })
    }

    fn term(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.combination()?;
        loop {
            let op = match self.peek() {
                Some(TokenKind::Star) => BinaryOp::Mul,
                Some(TokenKind::Slash) => BinaryOp::Div,
                // Implicit multiplication: a value directly followed by
                // another value, e.g. 2π, 2(3+4), (2)(3), 5sin(30), 2A.
                Some(_) if !self.options.implicit_mul_first && self.at_operand() => {
                    left = Expr::binary(BinaryOp::Mul, left, self.unary()?);
                    continue;
                }
                _ => return Ok(left),
            };
            self.advance();
            left = Expr::binary(op, left, self.combination()?);
        }
    }

    /// `nPr` / `nCr` between two values: `7nCr3` is `nCr(7, 3)`.
    fn combination(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.implicit()?;
        while let Some(func) = self.infix_func() {
            self.advance();
            let right = self.implicit()?;
            left = Expr::Call {
                func,
                args: vec![left, right],
            };
        }
        Ok(left)
    }

    /// The next token is `nPr` or `nCr` used as an operator (no `(` after it).
    fn infix_func(&self) -> Option<FuncId> {
        match self.peek() {
            Some(TokenKind::Func(func))
                if matches!(func.name(), "nPr" | "nCr")
                    && self.peek_at(1) != Some(TokenKind::LParen) =>
            {
                Some(func)
            }
            _ => None,
        }
    }

    /// Can the next token begin a value? Used for implicit multiplication.
    fn at_operand(&self) -> bool {
        self.peek().is_some_and(starts_operand) && self.infix_func().is_none()
    }

    /// With `implicit_mul_first`, a run of implicitly multiplied values;
    /// otherwise just one value.
    fn implicit(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.unary()?;
        if self.options.implicit_mul_first {
            while self.at_operand() {
                left = Expr::binary(BinaryOp::Mul, left, self.unary()?);
            }
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        self.nested(|p| match p.peek() {
            Some(TokenKind::Minus) => {
                p.advance();
                Ok(Expr::unary(UnaryOp::Neg, p.unary()?))
            }
            Some(TokenKind::Plus) => {
                p.advance();
                p.unary()
            }
            _ => p.power(),
        })
    }

    fn power(&mut self) -> Result<Expr, ParseError> {
        let base = self.postfix()?;
        if self.eat(TokenKind::Caret) {
            // `unary`, not `power`: allows 2^-1, and recursing to the right
            // makes ^ right-associative.
            let exponent = self.unary()?;
            return Ok(Expr::binary(BinaryOp::Pow, base, exponent));
        }
        if self.eat(TokenKind::XRoot) {
            let radicand = self.unary()?;
            let root = FuncId::lookup("root").expect("root is a built-in function");
            return Ok(Expr::Call {
                func: root,
                args: vec![radicand, base],
            });
        }
        Ok(base)
    }

    fn postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.primary()?;
        loop {
            let op = match self.peek() {
                Some(TokenKind::Bang) => PostfixOp::Factorial,
                Some(TokenKind::Percent) => PostfixOp::Percent,
                Some(TokenKind::Degree) => {
                    self.advance();
                    expr = self.dms(expr);
                    continue;
                }
                Some(mark @ (TokenKind::Radian | TokenKind::Gradian)) => {
                    self.advance();
                    let unit = if mark == TokenKind::Radian {
                        AngleMode::Radians
                    } else {
                        AngleMode::Gradians
                    };
                    expr = Expr::Angle {
                        unit,
                        value: Box::new(expr),
                    };
                    continue;
                }
                _ => return Ok(expr),
            };
            self.advance();
            expr = Expr::postfix(op, expr);
        }
    }

    /// After `°`: optional minutes and seconds, e.g. the `30′15″` of
    /// `12°30′15″`.
    fn dms(&mut self, degrees: Expr) -> Expr {
        let mut total = degrees;
        for (mark, per_degree) in [(TokenKind::Minute, 60.0), (TokenKind::Second, 3600.0)] {
            if let (Some(TokenKind::Number(n)), Some(kind)) = (self.peek(), self.peek_at(1))
                && kind == mark
            {
                self.pos += 2;
                total = Expr::binary(BinaryOp::Add, total, Expr::Number(n / per_degree));
            }
        }
        Expr::Angle {
            unit: AngleMode::Degrees,
            value: Box::new(total),
        }
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        let error = self.unexpected();
        let start = self.span().start;
        match self.advance() {
            Some(TokenKind::Number(n)) => Ok(Expr::Number(n)),
            Some(TokenKind::Constant(c)) => Ok(Expr::Constant(c)),
            Some(TokenKind::Variable(v)) => Ok(Expr::Variable(v)),
            Some(TokenKind::Unit(u)) => Ok(Expr::Unit(u)),
            Some(TokenKind::Func(func)) => self.call(func, start),
            Some(TokenKind::LBracket) => self.matrix(start),
            Some(TokenKind::LBrace) => self.list(),
            Some(TokenKind::LParen) => {
                let inner = self.value()?;
                self.close(TokenKind::RParen)?;
                Ok(inner)
            }
            _ => Err(error),
        }
    }

    /// After the function name: `(args…)`, a single bare argument as in
    /// `√9` or `sin 30°`, or nothing for functions that take no arguments.
    fn call(&mut self, func: FuncId, start: usize) -> Result<Expr, ParseError> {
        let def = func.def();
        let args = if self.eat(TokenKind::LParen) {
            let mut args = Vec::new();
            if !self.eat(TokenKind::RParen) {
                loop {
                    args.push(self.argument()?);
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }
                self.close(TokenKind::RParen)?;
            }
            args
        } else if def.max_args == 0 {
            Vec::new()
        } else {
            vec![self.nested(Self::bare_argument)?]
        };

        if !(def.min_args..=def.max_args).contains(&args.len()) {
            return Err(ParseError::WrongArgumentCount {
                span: start..self.last_end(),
            });
        }
        Ok(Expr::Call { func, args })
    }

    /// The argument of a function written without brackets. As on a
    /// calculator, it takes a sign, powers and postfix operators, but not
    /// × ÷ + −: `sin −30`, `log 10^3` is `log(10^3)`, `sin 30²` is
    /// `sin(30²)`, while `sin 30+1` is `sin(30) + 1`.
    fn bare_argument(&mut self) -> Result<Expr, ParseError> {
        if self.eat(TokenKind::Minus) {
            return Ok(Expr::unary(UnaryOp::Neg, self.nested(Self::bare_argument)?));
        }
        self.power()
    }

    fn argument(&mut self) -> Result<Expr, ParseError> {
        let left = self.value()?;
        if !self.eat(TokenKind::Equals) {
            return Ok(left);
        }
        let right = self.value()?;
        Ok(Expr::Equation {
            left: Box::new(left),
            right: Box::new(right),
        })
    }

    /// After `[`: rows separated by `;`, elements separated by `,`.
    fn matrix(&mut self, start: usize) -> Result<Expr, ParseError> {
        let mut rows = Vec::new();
        loop {
            let mut row = vec![self.expr()?];
            while self.eat(TokenKind::Comma) {
                row.push(self.expr()?);
            }
            rows.push(row);
            if !self.eat(TokenKind::Semicolon) {
                break;
            }
        }
        self.close(TokenKind::RBracket)?;

        if rows.iter().any(|r| r.len() != rows[0].len()) {
            return Err(ParseError::RaggedMatrix {
                span: start..self.last_end(),
            });
        }
        Ok(Expr::Matrix(rows))
    }

    /// After `{`: items separated by `,`.
    fn list(&mut self) -> Result<Expr, ParseError> {
        let mut items = Vec::new();
        if !self.eat(TokenKind::RBrace) {
            loop {
                items.push(self.value()?);
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.close(TokenKind::RBrace)?;
        }
        Ok(Expr::List(items))
    }
}

/// A statement that starts with an operator carries on from the last
/// result, as on a calculator: `×4` is `Ans×4`, and in `2+3:×4` the `×4`
/// uses 5. `−` is left alone: `−4` is negative four.
fn carry_on_from_ans(tokens: &[Token]) -> Vec<Token> {
    let mut out = Vec::with_capacity(tokens.len() + 1);
    for (i, token) in tokens.iter().enumerate() {
        let at_start = i == 0 || tokens[i - 1].kind == TokenKind::Colon;
        let infix_func = matches!(token.kind, TokenKind::Func(f) if matches!(f.name(), "nPr" | "nCr"))
            && tokens.get(i + 1).map(|t| t.kind) != Some(TokenKind::LParen);
        let operator = matches!(
            token.kind,
            TokenKind::Plus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Caret
                | TokenKind::XRoot
                | TokenKind::Bang
                | TokenKind::Percent
        );
        if at_start && (operator || infix_func) {
            out.push(Token {
                kind: TokenKind::Variable(Var::Ans),
                span: token.span.start..token.span.start,
            });
        }
        out.push(token.clone());
    }
    out
}

/// Can this token begin a value? Used for implicit multiplication.
fn starts_operand(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Number(_)
            | TokenKind::Constant(_)
            | TokenKind::Variable(_)
            | TokenKind::Func(_)
            | TokenKind::LParen
            | TokenKind::LBracket
            | TokenKind::LBrace
    )
}
