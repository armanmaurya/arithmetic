use crate::core::engine::lexer::{LexOptions, cursor::Cursor, error::LexError, token::TokenKind};

/// Takes every digit and dot, then lets Rust's `f64` parser decide whether
/// they form a number. `1.2.3` becomes one clear error instead of two numbers.
/// Numbers too large for `f64` are rejected too: Rust parses them as infinity.
///
/// Scientific notation: `1.5E3`, `2E−4`, `1ᴇ+6`. The exponent marker counts
/// only when digits follow it, so `2E` alone is still 2 × the variable E.
pub(in crate::core::engine::lexer) fn lex_number(
    cur: &mut Cursor,
    options: LexOptions,
) -> Result<TokenKind, LexError> {
    let start = cur.pos();
    if let Some(kind) = based(cur, options.word_bits) {
        return kind;
    }
    if options.radix != 10 {
        let len = digit_run(cur.rest(), options.radix);
        let text = &cur.rest()[..len];
        cur.eat_str(text);
        return integer(text, options.radix, options.word_bits, start..cur.pos());
    }
    cur.eat_while(|c| c.is_ascii_digit() || c == '.');
    let mut text = cur.slice_from(start).to_string();

    if let Some(exponent) = exponent(cur.rest()) {
        cur.eat_str(exponent);
        let marker_len = exponent.chars().next().map_or(0, char::len_utf8);
        text.push('e');
        text.push_str(&exponent[marker_len..].replace('−', "-"));
    }

    match text.parse::<f64>() {
        Ok(value) if value.is_finite() => Ok(TokenKind::Number(value)),
        _ => Err(LexError::BadNumber {
            span: start..cur.pos(),
        }),
    }
}

/// The exponent part at the start of `rest`, e.g. `E3`, `E−4` or `ᴇ+6`:
/// a marker, an optional sign, then at least one digit.
fn exponent(rest: &str) -> Option<&str> {
    let after_marker = rest.strip_prefix('E').or_else(|| rest.strip_prefix('ᴇ'))?;
    let after_sign = after_marker
        .strip_prefix(['+', '-', '−'])
        .unwrap_or(after_marker);
    let digits = after_sign.len()
        - after_sign
            .trim_start_matches(|c: char| c.is_ascii_digit())
            .len();
    if digits == 0 {
        return None;
    }
    let len = rest.len() - after_sign.len() + digits;
    Some(&rest[..len])
}

/// Whole numbers in a given base: `0x1F`, `0b101`, `0o17`, `0d15`. The prefix
/// counts only when a valid digit follows, so `0x` alone is still 0 × x.
fn based(cur: &mut Cursor, word_bits: Option<u8>) -> Option<Result<TokenKind, LexError>> {
    let start = cur.pos();
    let rest = cur.rest();
    let radix = match rest.get(..2)? {
        "0x" => 16,
        "0b" => 2,
        "0o" => 8,
        "0d" => 10,
        _ => return None,
    };
    let digits = rest[2..]
        .find(|c: char| !c.is_digit(radix))
        .unwrap_or(rest.len() - 2);
    if digits == 0 {
        return None;
    }
    let text = &rest[2..2 + digits];
    cur.eat_str(&rest[..2 + digits]);
    Some(integer(text, radix, word_bits, start..cur.pos()))
}

/// A whole number marked with its base by a letter: `d15`, `h1F`, `b101`,
/// `o17`. Only used in word mode; `None` (consuming nothing) if no digit
/// follows the letter.
pub(in crate::core::engine::lexer) fn lex_marked(
    cur: &mut Cursor,
    options: LexOptions,
) -> Option<Result<TokenKind, LexError>> {
    let start = cur.pos();
    let rest = cur.rest();
    let radix = match rest.chars().next()? {
        'd' => 10,
        'h' => 16,
        'b' => 2,
        'o' => 8,
        _ => return None,
    };
    let digits = digit_run(&rest[1..], radix);
    if digits == 0 {
        return None;
    }
    let text = &rest[1..1 + digits];
    cur.eat_str(&rest[..1 + digits]);
    Some(integer(text, radix, options.word_bits, start..cur.pos()))
}

/// How many bytes at the start of `rest` could be digits of a number in
/// `radix`: `0`–`9`, plus `A`–`F` in base 16. Digits too big for the base
/// are included so that `12` in binary is an error, not 1 then 2.
pub(in crate::core::engine::lexer) fn digit_run(rest: &str, radix: u32) -> usize {
    rest.find(|c: char| !(c.is_ascii_digit() || (radix == 16 && matches!(c, 'A'..='F'))))
        .unwrap_or(rest.len())
}

/// Digits in `radix` as a number, wrapped to a signed word if `word_bits`
/// is given.
fn integer(
    text: &str,
    radix: u32,
    word_bits: Option<u8>,
    span: std::ops::Range<usize>,
) -> Result<TokenKind, LexError> {
    let bad = || LexError::BadNumber { span: span.clone() };
    let n = u64::from_str_radix(text, radix).map_err(|_| bad())?;
    let value = match word_bits {
        None => n as f64,
        Some(bits) => {
            let n = n as i128;
            if n >= 1i128 << bits {
                return Err(bad());
            }
            let negative = n >= 1i128 << (bits - 1);
            (if negative { n - (1i128 << bits) } else { n }) as f64
        }
    };
    Ok(TokenKind::Number(value))
}
