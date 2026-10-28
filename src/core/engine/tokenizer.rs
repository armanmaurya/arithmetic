#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(f64),
    Plus,
    Minus,
    Multiply,
    Divide,
    Power,
    Percent,
    Factorial,
    LParen,
    RParen,
    Func(String),
}

pub fn tokenize(expr: &str) -> Result<Vec<Token>, String> {
    // Normalize inverse trig superscripts so tokenizer handles them cleanly
    let normalized = expr
        .replace("sin⁻¹", "asin")
        .replace("cos⁻¹", "acos")
        .replace("tan⁻¹", "atan");

    let mut raw_tokens = Vec::new();
    let chars: Vec<char> = normalized.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }

        if c.is_ascii_digit() || c == '.' {
            let start = i;
            let mut dot_count = 0;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                if chars[i] == '.' {
                    dot_count += 1;
                }
                i += 1;
            }
            if dot_count > 1 {
                return Err("Format error".into());
            }
            let num_str: String = chars[start..i].iter().collect();
            if num_str == "." {
                raw_tokens.push(Token::Number(0.0));
            } else {
                let val = num_str.parse::<f64>().map_err(|_| "Format error")?;
                raw_tokens.push(Token::Number(val));
            }
            continue;
        }

        match c {
            '+' => {
                raw_tokens.push(Token::Plus);
                i += 1;
            }
            '-' | '−' => {
                raw_tokens.push(Token::Minus);
                i += 1;
            }
            '*' | '×' => {
                raw_tokens.push(Token::Multiply);
                i += 1;
            }
            '/' | '÷' => {
                raw_tokens.push(Token::Divide);
                i += 1;
            }
            '^' => {
                raw_tokens.push(Token::Power);
                i += 1;
            }
            '%' => {
                raw_tokens.push(Token::Percent);
                i += 1;
            }
            '!' => {
                raw_tokens.push(Token::Factorial);
                i += 1;
            }
            '(' => {
                raw_tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                raw_tokens.push(Token::RParen);
                i += 1;
            }
            'π' => {
                raw_tokens.push(Token::Number(std::f64::consts::PI));
                i += 1;
            }
            'e' => {
                raw_tokens.push(Token::Number(std::f64::consts::E));
                i += 1;
            }
            '√' => {
                raw_tokens.push(Token::Func("sqrt".into()));
                i += 1;
            }
            _ if c.is_ascii_alphabetic() => {
                let start = i;
                while i < chars.len() && chars[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let name: String = chars[start..i].iter().collect();
                raw_tokens.push(Token::Func(name));
            }
            _ => return Err("Format error".into()),
        }
    }

    // Insert implicit multiplication (e.g. 2π, 2(3+4), (2)(3), 5sin(30))
    let mut tokens = Vec::with_capacity(raw_tokens.len() * 2);
    for idx in 0..raw_tokens.len() {
        if idx > 0 {
            let prev = &raw_tokens[idx - 1];
            let curr = &raw_tokens[idx];
            let prev_is_value = matches!(
                prev,
                Token::Number(_) | Token::RParen | Token::Percent | Token::Factorial
            );
            let curr_starts_value =
                matches!(curr, Token::Number(_) | Token::LParen | Token::Func(_));
            if prev_is_value && curr_starts_value {
                tokens.push(Token::Multiply);
            }
        }
        tokens.push(raw_tokens[idx].clone());
    }

    Ok(tokens)
}
