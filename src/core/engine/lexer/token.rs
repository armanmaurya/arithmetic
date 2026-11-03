//! What the lexer produces. Data only, no logic.

pub use crate::core::engine::{constants::ConstId, functions::FuncId, units::UnitId};
use std::ops::Range;

pub type Span = Range<usize>;

/// Names that hold a value. `Ans` is the last result and is read-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Var {
    A,
    B,
    C,
    D,
    E,
    F,
    X,
    Y,
    M,
    Ans,
}

impl Var {
    pub const ALL: [Self; 10] = [
        Self::A,
        Self::B,
        Self::C,
        Self::D,
        Self::E,
        Self::F,
        Self::X,
        Self::Y,
        Self::M,
        Self::Ans,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::E => "E",
            Self::F => "F",
            Self::X => "X",
            Self::Y => "Y",
            Self::M => "M",
            Self::Ans => "ans",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenKind {
    Number(f64),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Percent,
    Bang,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Semicolon,
    /// `=`, only meaningful inside `solve(…)`.
    Equals,
    /// `→` or `->`: store a value, e.g. `5→A`.
    Store,
    LBrace,
    RBrace,
    /// `:` separates statements: `1→A : A+1`.
    Colon,
    /// `°`, after a number in degrees.
    Degree,
    /// `ʳ`, after a number in radians.
    Radian,
    /// `ᵍ`, after a number in gradians.
    Gradian,
    /// `∠`, polar complex numbers: `2∠30` has length 2 and angle 30.
    Angle,
    /// `ˣ√`, the x-th root written between its operands: `3ˣ√27` is 3.
    XRoot,
    /// `′` or `'`, minutes of arc in `12°30′`.
    Minute,
    /// `″` or `"`, seconds of arc in `12°30′15″`.
    Second,
    Func(FuncId),
    Constant(ConstId),
    /// `_km`, `_degC`, …: only meaningful inside `convert`.
    Unit(UnitId),
    Variable(Var),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}
