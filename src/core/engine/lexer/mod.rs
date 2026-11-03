mod cursor;
mod error;
mod rules;
mod token;

#[cfg(test)]
mod tests;

pub use error::LexError;
pub use token::{ConstId, FuncId, Span, Token, TokenKind, UnitId, Var};

use cursor::Cursor;

/// How numbers are read. The default is plain decimal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LexOptions {
    /// Base of number literals without a prefix: 2, 8, 10 or 16. In base
    /// 16, `A`–`F` are digits (`FF` is 255), unless they start a longer
    /// name such as `Ans`.
    pub radix: u32,
    /// `Some(bits)`: literals in other bases are two's-complement words,
    /// so `0xFF` with 8 bits is −1. Literals too big for the word fail.
    pub word_bits: Option<u8>,
}

impl Default for LexOptions {
    fn default() -> Self {
        Self {
            radix: 10,
            word_bits: None,
        }
    }
}

pub fn tokenize(src: &str) -> Result<Vec<Token>, LexError> {
    tokenize_with(src, LexOptions::default())
}

pub fn tokenize_with(src: &str, options: LexOptions) -> Result<Vec<Token>, LexError> {
    let mut cur = Cursor::new(src);
    let mut tokens = Vec::new();

    while let Some(c) = cur.peek() {
        let start = cur.pos();

        let kind = match c {
            c if c.is_whitespace() => {
                cur.bump();
                continue;
            }
            '0'..='9' | '.' => rules::lex_number(&mut cur, options)?,
            // A hex digit, unless a longer name starts here.
            'A'..='F'
                if options.radix == 16
                    && rules::digit_run(cur.rest(), 16) >= rules::name_len(cur.rest()) =>
            {
                rules::lex_number(&mut cur, options)?
            }
            // In word mode, d / h / b / o before digits gives their base:
            // h1F is 31, as a calculator's BASE mode writes it.
            'd' | 'h' | 'b' | 'o' if options.word_bits.is_some() => {
                match rules::lex_marked(&mut cur, options) {
                    Some(kind) => kind?,
                    None => {
                        rules::lex_symbol(&mut cur).map_or_else(|| rules::lex_name(&mut cur), Ok)?
                    }
                }
            }
            _ => match rules::lex_symbol(&mut cur) {
                Some(kind) => kind,
                None => rules::lex_name(&mut cur)?,
            },
        };

        tokens.push(Token {
            kind,
            span: start..cur.pos(),
        });
    }

    Ok(tokens)
}
