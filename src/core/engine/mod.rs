//! The calculator engine: text in, value out, in three stages.
//!
//! ```text
//! "2+3×4" → lexer → tokens → parser → tree → evaluator → 14
//! ```
//!
//! The engine imitates no particular calculator. What differs between
//! calculators (angle unit, display format, how `%` and implicit `×`
//! behave, complex results, word size, …) is a [`Settings`] value, and
//! every feature is reachable as plain text, so any front end can use all
//! of it by sending text.
//!
//! Extending it:
//! - a function: one entry in a `functions/*.rs` table
//! - a constant: one line in `constants.rs`
//! - a unit: one line in `units.rs`
//! - a calculator difference: one field and key in `settings.rs`

pub mod constants;
pub mod context;
pub mod error;
pub mod evaluator;
pub mod format;
pub mod functions;
pub mod lexer;
pub mod numeric;
pub mod parser;
pub mod settings;
pub mod units;
pub mod value;

pub use context::Context;
pub use error::CalcError;
pub use evaluator::{EvalError, evaluate};
pub use format::{format_number, format_value, format_with};
pub use lexer::{LexError, LexOptions, Span, Token, TokenKind, Var, tokenize, tokenize_with};
pub use parser::{BinaryOp, Expr, ParseError, ParseOptions, PostfixOp, UnaryOp, parse, parse_with};
pub use settings::{
    AngleMode, Base, DisplaySettings, Notation, PercentStyle, SettingError, Settings,
};
pub use value::{Complex, Matrix, Value};

/// Evaluates one line of input and records it: the result becomes `ans`,
/// and `→` assignments and `solve` change variables.
pub fn evaluate_expression(input: &str, ctx: &mut Context) -> Result<Value, CalcError> {
    if input.trim().is_empty() {
        return Err(CalcError::Empty);
    }
    let settings = &ctx.settings;
    let lex = LexOptions {
        radix: settings.input_base.radix(),
        word_bits: settings.word_mode.then_some(settings.word_bits),
    };
    let tokens = tokenize_with(input, lex)?;
    let options = ParseOptions {
        implicit_mul_first: ctx.settings.implicit_mul_first,
    };
    let expr = parse_with(&tokens, options)?;
    ctx.reset_work();
    // Hide floating-point noise, e.g. 0.1 + 0.2 − 0.3 = 5.6E-17.
    let value = evaluate(&expr, ctx)?.tidy();
    ctx.set(Var::Ans, value.clone());
    Ok(value)
}

/// The result `input` would give, without changing anything, e.g. for a
/// live preview while typing.
pub fn preview_expression(input: &str, ctx: &Context) -> Result<Value, CalcError> {
    evaluate_expression(input, &mut ctx.clone())
}
