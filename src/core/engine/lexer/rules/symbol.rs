use crate::core::engine::lexer::{cursor::Cursor, token::TokenKind};

/// Symbols longer than one character. Checked before the single ones.
const MULTI: &[(&str, TokenKind)] = &[
    ("->", TokenKind::Store),
    ("**", TokenKind::Caret),
    ("ˣ√", TokenKind::XRoot),
];

/// To add an operator, add a line here (and a variant in `token.rs`).
fn symbol_kind(c: char) -> Option<TokenKind> {
    let kind = match c {
        '+' => TokenKind::Plus,
        '-' | '−' => TokenKind::Minus,
        '*' | '×' | '·' => TokenKind::Star,
        '/' | '÷' => TokenKind::Slash,
        '^' => TokenKind::Caret,
        '%' => TokenKind::Percent,
        '!' => TokenKind::Bang,
        '(' => TokenKind::LParen,
        ')' => TokenKind::RParen,
        '[' => TokenKind::LBracket,
        ']' => TokenKind::RBracket,
        ',' => TokenKind::Comma,
        ';' => TokenKind::Semicolon,
        '=' => TokenKind::Equals,
        '→' => TokenKind::Store,
        '{' => TokenKind::LBrace,
        '}' => TokenKind::RBrace,
        ':' => TokenKind::Colon,
        '°' => TokenKind::Degree,
        'ʳ' => TokenKind::Radian,
        'ᵍ' => TokenKind::Gradian,
        '∠' => TokenKind::Angle,
        '′' | '\'' => TokenKind::Minute,
        '″' | '"' => TokenKind::Second,
        _ => return None,
    };
    Some(kind)
}

/// Consumes one operator or bracket, or returns `None` and consumes nothing.
pub(in crate::core::engine::lexer) fn lex_symbol(cur: &mut Cursor) -> Option<TokenKind> {
    for (text, kind) in MULTI {
        if cur.eat_str(text) {
            return Some(*kind);
        }
    }
    let kind = symbol_kind(cur.peek()?)?;
    cur.bump();
    Some(kind)
}
