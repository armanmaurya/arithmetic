#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyAction {
    Clear,
    Evaluate,
    UpdateExpression {
        new_expression: String,
        new_cursor: usize,
    },
}

pub fn clamp_char_boundary(s: &str, byte_offset: usize) -> usize {
    let mut idx = byte_offset.min(s.len());
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
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

pub fn apply_key_press(
    expr: &str,
    cursor: usize,
    key_str: &str,
    just_eval: bool,
) -> KeyAction {
    match key_str {
        "AC" => KeyAction::Clear,
        "=" => KeyAction::Evaluate,
        "BACKSPACE" => {
            let (before, after) = expr.split_at(cursor);
            let mut before_str = before.to_string();
            pop_last_token(&mut before_str);
            let new_cursor = before_str.len();
            let new_expression = format!("{before_str}{after}");
            KeyAction::UpdateExpression {
                new_expression,
                new_cursor,
            }
        }
        "()" => {
            let (mut before_str, after_str) = if just_eval && cursor == expr.len() {
                (String::new(), "")
            } else {
                let (b, a) = expr.split_at(cursor);
                (b.to_string(), a)
            };
            let open_count = before_str.chars().filter(|&c| c == '(').count();
            let close_count = before_str.chars().filter(|&c| c == ')').count();
            let last_char = before_str.chars().last();
            let can_close = open_count > close_count
                && matches!(
                    last_char,
                    Some('0'..='9' | ')' | 'π' | 'e' | '%' | '!')
                );
            if can_close {
                before_str.push(')');
            } else {
                before_str.push('(');
            }
            let new_cursor = before_str.len();
            let new_expression = format!("{before_str}{after_str}");
            KeyAction::UpdateExpression {
                new_expression,
                new_cursor,
            }
        }
        _ => {
            let is_binary_op = matches!(key_str, "+" | "−" | "×" | "÷" | "^" | "^2");
            let is_digit_or_dot =
                key_str.len() == 1 && key_str.chars().all(|c| c.is_ascii_digit() || c == '.');

            let (mut before_str, after_str) = if just_eval
                && cursor == expr.len()
                && (is_digit_or_dot || key_str.ends_with('(') || key_str == "π" || key_str == "e")
            {
                (String::new(), "")
            } else {
                let (b, a) = expr.split_at(cursor);
                (b.to_string(), a)
            };

            if is_binary_op {
                if before_str.is_empty() {
                    if key_str == "−" {
                        before_str.push('−');
                    }
                } else if let Some(last) = before_str.chars().last() {
                    if matches!(last, '+' | '−' | '×' | '÷' | '^') {
                        before_str.pop();
                        before_str.push_str(key_str);
                    } else if last != '(' || key_str == "−" {
                        before_str.push_str(key_str);
                    }
                }
            } else if key_str == "." {
                let last_number_segment = before_str
                    .rsplit(|c: char| !c.is_ascii_digit() && c != '.')
                    .next()
                    .unwrap_or("");
                if !last_number_segment.contains('.') {
                    if last_number_segment.is_empty() {
                        before_str.push('0');
                    }
                    before_str.push('.');
                }
            } else {
                before_str.push_str(key_str);
            }

            let new_cursor = before_str.len();
            let new_expression = format!("{before_str}{after_str}");
            KeyAction::UpdateExpression {
                new_expression,
                new_cursor,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pop_last_token() {
        let mut s = "sin(".to_string();
        pop_last_token(&mut s);
        assert_eq!(s, "");

        let mut s = "sin⁻¹(".to_string();
        pop_last_token(&mut s);
        assert_eq!(s, "");

        let mut s = "42".to_string();
        pop_last_token(&mut s);
        assert_eq!(s, "4");
    }

    #[test]
    fn test_parenthesis_balancing() {
        let action = apply_key_press("", 0, "()", false);
        assert_eq!(
            action,
            KeyAction::UpdateExpression {
                new_expression: "(".into(),
                new_cursor: 1,
            }
        );

        let action = apply_key_press("(5", 2, "()", false);
        assert_eq!(
            action,
            KeyAction::UpdateExpression {
                new_expression: "(5)".into(),
                new_cursor: 3,
            }
        );
    }

    #[test]
    fn test_operator_deduplication() {
        let action = apply_key_press("5+", 2, "×", false);
        assert_eq!(
            action,
            KeyAction::UpdateExpression {
                new_expression: "5×".into(),
                new_cursor: "5×".len(),
            }
        );
    }
}
