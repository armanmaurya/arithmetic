use super::token::Span;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum LexError {
    UnexpectedChar { ch: char, span: Span },
    UnknownName { name: String, span: Span },
    BadNumber { span: Span },
}

impl LexError {
    /// Where in the input the error is, e.g. for highlighting it in the UI.
    pub fn span(&self) -> Span {
        match self {
            Self::UnexpectedChar { span, .. }
            | Self::UnknownName { span, .. }
            | Self::BadNumber { span } => span.clone(),
        }
    }
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedChar { ch, span } => {
                write!(
                    f,
                    "unexpected character '{ch}' at bytes {}..{}",
                    span.start, span.end
                )
            }
            Self::UnknownName { name, span } => {
                write!(
                    f,
                    "unknown name '{name}' at bytes {}..{}",
                    span.start, span.end
                )
            }
            Self::BadNumber { span } => {
                write!(f, "invalid number at bytes {}..{}", span.start, span.end)
            }
        }
    }
}

impl std::error::Error for LexError {}
