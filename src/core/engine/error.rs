use super::{evaluator::EvalError, lexer::LexError, parser::ParseError};
use std::fmt;

/// Any reason `evaluate_expression` can fail, keeping the stage's own error.
#[derive(Debug, Clone, PartialEq)]
pub enum CalcError {
    /// Blank input. The UI shows nothing rather than an error.
    Empty,
    Lex(LexError),
    Parse(ParseError),
    Eval(EvalError),
}

/// The message the calculator screen shows. Lexing and parsing problems
/// share one message; the wrapped error has the details.
impl fmt::Display for CalcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => Ok(()),
            Self::Lex(_) | Self::Parse(_) => f.write_str("Format error"),
            Self::Eval(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for CalcError {}

impl From<LexError> for CalcError {
    fn from(e: LexError) -> Self {
        Self::Lex(e)
    }
}

impl From<ParseError> for CalcError {
    fn from(e: ParseError) -> Self {
        Self::Parse(e)
    }
}

impl From<EvalError> for CalcError {
    fn from(e: EvalError) -> Self {
        Self::Eval(e)
    }
}
