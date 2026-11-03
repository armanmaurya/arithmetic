use crate::core::engine::lexer::Span;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// A token where the grammar doesn't allow one, e.g. the `×` in `2+×3`.
    UnexpectedToken { span: Span },
    /// The input stopped where a value was still expected, e.g. `2+`.
    /// The span is empty and sits at the end of the input.
    UnexpectedEnd { span: Span },
    /// A `)` or `]` with nothing to close.
    UnmatchedBracket { span: Span },
    /// A function called with too few or too many arguments, e.g. `nCr(5)`.
    WrongArgumentCount { span: Span },
    /// Matrix rows of different lengths, e.g. `[1, 2; 3]`.
    RaggedMatrix { span: Span },
    /// Brackets or signs nested deeper than the parser allows.
    TooDeep { span: Span },
}

impl ParseError {
    /// Where in the input the error is, e.g. for highlighting it in the UI.
    pub fn span(&self) -> Span {
        match self {
            Self::UnexpectedToken { span }
            | Self::UnexpectedEnd { span }
            | Self::UnmatchedBracket { span }
            | Self::WrongArgumentCount { span }
            | Self::RaggedMatrix { span }
            | Self::TooDeep { span } => span.clone(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let span = self.span();
        let what = match self {
            Self::UnexpectedToken { .. } => "unexpected token",
            Self::UnexpectedEnd { .. } => "unexpected end of input",
            Self::UnmatchedBracket { .. } => "unmatched closing bracket",
            Self::WrongArgumentCount { .. } => "wrong number of arguments",
            Self::RaggedMatrix { .. } => "matrix rows have different lengths",
            Self::TooDeep { .. } => "nested too deeply",
        };
        write!(f, "{what} at bytes {}..{}", span.start, span.end)
    }
}

impl std::error::Error for ParseError {}
