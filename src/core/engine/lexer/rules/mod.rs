//! One file per kind of token. Each rule reads one token from the cursor.
//!
//! To support a new kind of token, add a file here and call it from
//! `tokenize` in `lexer/mod.rs`.

mod name;
mod number;
mod symbol;

pub(super) use name::{lex_name, name_len};
pub(super) use number::{digit_run, lex_marked, lex_number};
pub(super) use symbol::lex_symbol;
