use crate::core::engine::lexer::{
    cursor::Cursor,
    error::LexError,
    token::{ConstId, FuncId, TokenKind, UnitId, Var},
};
use std::sync::OnceLock;

/// Every name in every spelling, longest first, built once from the
/// function and constant tables plus the variables. Case matters: `e` is
/// the constant, `E` the variable.
fn names() -> &'static [(&'static str, TokenKind)] {
    static NAMES: OnceLock<Vec<(&'static str, TokenKind)>> = OnceLock::new();
    NAMES.get_or_init(|| {
        let mut names: Vec<(&'static str, TokenKind)> = FuncId::all_names()
            .map(|(n, id)| (n, TokenKind::Func(id)))
            .chain(
                ConstId::all_names()
                    .filter(|(n, _)| !n.starts_with('_'))
                    .map(|(n, id)| (n, TokenKind::Constant(id))),
            )
            .chain(Var::ALL.iter().map(|&v| (v.name(), TokenKind::Variable(v))))
            .chain([
                ("x", TokenKind::Variable(Var::X)),
                ("y", TokenKind::Variable(Var::Y)),
                ("Ans", TokenKind::Variable(Var::Ans)),
            ])
            .collect();
        names.sort_by_key(|(n, _)| std::cmp::Reverse(n.len()));
        names
    })
}

/// Maximal munch: of all names `rest` starts with, the longest one. So
/// `sinh` is one name rather than `sin` then `h`, while `sinx` is `sin`
/// then `x`.
fn longest_name(rest: &str) -> Option<(&'static str, TokenKind)> {
    names()
        .iter()
        .find(|(name, _)| rest.starts_with(name))
        .copied()
}

/// Length in bytes of the longest name at the start of `rest` (0 if none).
pub(in crate::core::engine::lexer) fn name_len(rest: &str) -> usize {
    longest_name(rest).map_or(0, |(name, _)| name.len())
}

/// A name starting with `_` (physical constants and units) is a whole word
/// and must match exactly: `_mx` is an error, not `_m` then `x`.
fn underscore_name(word: &str) -> Option<TokenKind> {
    ConstId::lookup(word)
        .map(TokenKind::Constant)
        .or_else(|| UnitId::lookup(word).map(TokenKind::Unit))
}

pub(in crate::core::engine::lexer) fn lex_name(cur: &mut Cursor) -> Result<TokenKind, LexError> {
    let start = cur.pos();

    if cur.peek() == Some('_') {
        cur.bump();
        cur.eat_while(char::is_alphanumeric);
        let word = cur.slice_from(start);
        return underscore_name(word).ok_or_else(|| LexError::UnknownName {
            name: word.to_string(),
            span: start..cur.pos(),
        });
    }

    if let Some((name, kind)) = longest_name(cur.rest()) {
        cur.eat_str(name);
        return Ok(kind);
    }

    match cur.peek() {
        Some(c) if c.is_alphabetic() => {
            cur.eat_while(char::is_alphabetic);
            Err(LexError::UnknownName {
                name: cur.slice_from(start).to_string(),
                span: start..cur.pos(),
            })
        }
        Some(ch) => Err(LexError::UnexpectedChar {
            ch,
            span: start..start + ch.len_utf8(),
        }),
        None => unreachable!("lex_name is only called when a character is left"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_name_means_two_things() {
        let mut all: Vec<&str> = names().iter().map(|(n, _)| *n).collect();
        all.extend(
            crate::core::engine::constants::CONSTANTS
                .iter()
                .filter(|c| c.name.starts_with('_'))
                .map(|c| c.name),
        );
        all.extend(crate::core::engine::units::UNITS.iter().map(|u| u.name));
        let len = all.len();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), len, "two tables use the same name");
    }
}
