pub mod formatter;
pub mod parser;
pub mod tokenizer;

pub use formatter::format_number;
pub use parser::Parser;
pub use tokenizer::{tokenize, Token};

pub fn evaluate_expression(expr: &str, use_radians: bool) -> Result<f64, String> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return Err(String::new());
    }
    let tokens = tokenize(trimmed)?;
    if tokens.is_empty() {
        return Err(String::new());
    }
    let mut parser = Parser::new(tokens, use_radians);
    let result = parser.parse_expr()?;
    if parser.pos < parser.tokens.len() {
        return Err("Format error".into());
    }
    if result.is_nan() || result.is_infinite() {
        return Err("Value error".into());
    }
    let cleaned = if result.abs() < 1e-12 { 0.0 } else { result };
    Ok(cleaned)
}
