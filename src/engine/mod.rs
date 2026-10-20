pub mod parser;
pub mod tokenizer;

use parser::Parser;
use tokenizer::tokenize;

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

pub fn format_number(val: f64) -> String {
    if val == 0.0 {
        return "0".into();
    }
    let abs = val.abs();
    if !(1e-7..1e12).contains(&abs) {
        return format!("{val:.6e}");
    }
    let rounded = (val * 1e10).round() / 1e10;
    if (rounded - rounded.trunc()).abs() < 1e-10 {
        format!("{}", rounded as i64)
    } else {
        let s = format!("{rounded:.10}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

pub fn has_operator_or_func(expr: &str) -> bool {
    expr.chars().skip(1).any(|c| {
        matches!(
            c,
            '+' | '−' | '-' | '×' | '*' | '÷' | '/' | '^' | '%' | '!' | '√' | 'π' | 'e' | '(' | ')'
        ) || c.is_ascii_alphabetic()
    }) || expr.starts_with('√')
        || expr.starts_with("sin")
        || expr.starts_with("cos")
        || expr.starts_with("tan")
        || expr.starts_with("ln")
        || expr.starts_with("log")
        || expr.contains('π')
        || expr.contains('e')
        || expr.contains('!')
        || expr.contains('%')
}

pub fn pop_last_token(expr: &mut String) {
    for suffix in [
        "sin⁻¹(",
        "cos⁻¹(",
        "tan⁻¹(",
        "sin(",
        "cos(",
        "tan(",
        "log(",
        "ln(",
        "10^(",
        "e^(",
        "√(",
    ] {
        if expr.ends_with(suffix) {
            let new_len = expr.len() - suffix.len();
            expr.truncate(new_len);
            return;
        }
    }
    expr.pop();
}
