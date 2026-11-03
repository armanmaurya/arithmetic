use crate::core::engine::{TokenKind, Var, tokenize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyAction {
    Clear,
    Evaluate,
    /// `M+` / `M−`: evaluate, then add the result to (or subtract it from) `M`.
    MemoryAdd,
    MemorySubtract,
    /// `MC`: clear `M`.
    MemoryClear,
    /// `CLEAR-VARIABLES`: clear every variable, `ans` and memory.
    ClearVariables,
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

/// Removes the last token before the cursor: a whole name such as `sin(`,
/// `ans` or `_km` at once, otherwise one character.
pub fn pop_last_token(expr: &mut String) {
    for suffix in ["10^(", "e^("] {
        if let Some(rest) = expr.strip_suffix(suffix) {
            expr.truncate(rest.len());
            return;
        }
    }
    if let Ok(tokens) = tokenize(expr) {
        let is_name = |kind: TokenKind| {
            matches!(
                kind,
                TokenKind::Func(_)
                    | TokenKind::Constant(_)
                    | TokenKind::Unit(_)
                    | TokenKind::Variable(Var::Ans)
            )
        };
        let name_start = match tokens.as_slice() {
            // "sin(" → remove the name and its bracket together
            [.., name, paren]
                if paren.kind == TokenKind::LParen
                    && is_name(name.kind)
                    && name.span.end == paren.span.start
                    && paren.span.end == expr.len() =>
            {
                Some(name.span.start)
            }
            [.., name] if is_name(name.kind) && name.span.end == expr.len() => {
                Some(name.span.start)
            }
            _ => None,
        };
        if let Some(start) = name_start {
            expr.truncate(start);
            return;
        }
    }
    expr.pop();
}

/// The byte length of the first token of `after`: a whole name such as
/// `sin(` or `ans` at once, otherwise one character. Used to move the
/// cursor right.
fn next_token_len(after: &str) -> usize {
    let one_char = after.chars().next().map_or(0, char::len_utf8);
    let Ok(tokens) = tokenize(after) else {
        return one_char;
    };
    let is_name = |kind: TokenKind| {
        matches!(
            kind,
            TokenKind::Func(_)
                | TokenKind::Constant(_)
                | TokenKind::Unit(_)
                | TokenKind::Variable(Var::Ans)
        )
    };
    match tokens.as_slice() {
        [name, paren, ..]
            if name.span.start == 0
                && is_name(name.kind)
                && paren.kind == TokenKind::LParen
                && paren.span.start == name.span.end =>
        {
            paren.span.end
        }
        [name, ..] if name.span.start == 0 && is_name(name.kind) => name.span.end,
        _ => one_char,
    }
}

/// Whether `expr` is more than a plain number, i.e. worth previewing and
/// keeping in the history.
pub fn has_operator_or_func(expr: &str) -> bool {
    match tokenize(expr) {
        Ok(tokens) => {
            let mut kinds = tokens.iter().map(|t| t.kind).peekable();
            kinds.next_if_eq(&TokenKind::Minus);
            !kinds.all(|k| matches!(k, TokenKind::Number(_)))
        }
        // Unknown text: let evaluation report it.
        Err(_) => true,
    }
}

/// Whether a key starts a new value, so that pressing it right after `=`
/// replaces the result instead of extending it: digits, brackets, and any
/// function, constant or variable name.
fn starts_value(key: &str) -> bool {
    let Some(first) = key.chars().next() else {
        return false;
    };
    if first.is_ascii_digit() || first == '.' || key.ends_with('(') || matches!(first, '[' | '{') {
        return true;
    }
    // "E" alone is left out: after a result it more likely means ×10ⁿ.
    key != "E"
        && tokenize(key).is_ok_and(|tokens| {
            tokens.first().is_some_and(|t| {
                matches!(
                    t.kind,
                    TokenKind::Func(_) | TokenKind::Constant(_) | TokenKind::Variable(_)
                )
            })
        })
}

pub fn apply_key_press(expr: &str, cursor: usize, key_str: &str, just_eval: bool) -> KeyAction {
    match key_str {
        "AC" => KeyAction::Clear,
        "=" => KeyAction::Evaluate,
        "M+" => KeyAction::MemoryAdd,
        "M−" | "M-" => KeyAction::MemorySubtract,
        "MC" => KeyAction::MemoryClear,
        "CLEAR-VARIABLES" => KeyAction::ClearVariables,
        "LEFT" | "RIGHT" | "HOME" | "END" => {
            let new_cursor = match key_str {
                "LEFT" => {
                    let mut before = expr[..cursor].to_string();
                    pop_last_token(&mut before);
                    before.len()
                }
                "RIGHT" => cursor + next_token_len(&expr[cursor..]),
                "HOME" => 0,
                _ => expr.len(),
            };
            KeyAction::UpdateExpression {
                new_expression: expr.to_string(),
                new_cursor,
            }
        }
        "MR" => apply_key_press(expr, cursor, "M", just_eval),
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
                && matches!(last_char, Some('0'..='9' | ')' | 'π' | 'e' | '%' | '!'));
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
                && (is_digit_or_dot || starts_value(key_str))
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

        for (before, after) in [
            ("2×linsolve(", "2×"),
            ("3+ans", "3+"),
            ("convert(5, _km", "convert(5, "),
            ("sin(30", "sin(3"),
            ("2e^(", "2"),
        ] {
            let mut s = before.to_string();
            pop_last_token(&mut s);
            assert_eq!(s, after, "{before}");
        }
    }

    #[test]
    fn cursor_moves_over_whole_names() {
        let cursor = |expr: &str, at: usize, key: &str| match apply_key_press(expr, at, key, false)
        {
            KeyAction::UpdateExpression { new_cursor, .. } => new_cursor,
            other => panic!("{other:?}"),
        };
        let e = "2+sin(30)";
        assert_eq!(cursor(e, e.len(), "LEFT"), e.len() - 1);
        assert_eq!(cursor(e, 6, "LEFT"), 2);
        assert_eq!(cursor(e, 2, "RIGHT"), 6);
        assert_eq!(cursor(e, 0, "RIGHT"), 1);
        assert_eq!(cursor(e, 4, "HOME"), 0);
        assert_eq!(cursor(e, 4, "END"), e.len());
        assert_eq!(cursor(e, e.len(), "RIGHT"), e.len());
        assert_eq!(cursor("π", 0, "RIGHT"), "π".len());
    }

    #[test]
    fn plain_numbers_are_not_previewed() {
        assert!(!has_operator_or_func("42"));
        assert!(!has_operator_or_func("-4.5"));
        assert!(!has_operator_or_func("1.5E10"));
        assert!(has_operator_or_func("2+3"));
        assert!(has_operator_or_func("π"));
        assert!(has_operator_or_func("sin(30"));
        assert!(has_operator_or_func("{1, 2}"));
    }

    #[test]
    fn fresh_start_after_evaluation() {
        let fresh = |key: &str| apply_key_press("42", 2, key, true);
        let expr = |a: KeyAction| match a {
            KeyAction::UpdateExpression { new_expression, .. } => new_expression,
            other => panic!("{other:?}"),
        };
        assert_eq!(expr(fresh("7")), "7");
        assert_eq!(expr(fresh("sin(")), "sin(");
        assert_eq!(expr(fresh("A")), "A");
        assert_eq!(expr(fresh("π")), "π");
        assert_eq!(expr(fresh("+")), "42+");
        assert_eq!(expr(fresh("E")), "42E");
        assert_eq!(expr(fresh("MR")), "M");
        assert_eq!(fresh("M+"), KeyAction::MemoryAdd);
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
