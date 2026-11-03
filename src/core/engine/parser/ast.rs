//! What the parser produces: a tree that records which operation applies to
//! what. Data only, no maths.

use crate::core::engine::{
    constants::ConstId, functions::FuncId, lexer::Var, settings::AngleMode, units::UnitId,
};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    Constant(ConstId),
    Variable(Var),
    /// A unit name such as `_km`, only meaningful inside `convert`.
    Unit(UnitId),
    /// `[1, 2; 3, 4]`: rows of elements, all rows the same length.
    Matrix(Vec<Vec<Expr>>),
    /// `{1, 2, 3}`.
    List(Vec<Expr>),
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Postfix {
        op: PostfixOp,
        operand: Box<Expr>,
    },
    /// An angle with its unit marked (`30°`, `12°30′15″`, `1ʳ`, `100ᵍ`),
    /// converted to the current angle unit when evaluated.
    Angle {
        unit: AngleMode,
        value: Box<Expr>,
    },
    Call {
        func: FuncId,
        args: Vec<Expr>,
    },
    /// `left = right`, only allowed as an argument (for `solve`).
    Equation {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// `value → target`.
    Assign {
        target: Var,
        value: Box<Expr>,
    },
    /// `a : b : c`: evaluates each in turn; the result is the last one.
    Sequence(Vec<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    /// `r∠θ`: the complex number with length r and angle θ.
    Polar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostfixOp {
    Factorial,
    Percent,
}

impl Expr {
    pub(super) fn unary(op: UnaryOp, operand: Expr) -> Self {
        Self::Unary {
            op,
            operand: Box::new(operand),
        }
    }

    pub(super) fn binary(op: BinaryOp, left: Expr, right: Expr) -> Self {
        Self::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    pub(super) fn postfix(op: PostfixOp, operand: Expr) -> Self {
        Self::Postfix {
            op,
            operand: Box::new(operand),
        }
    }
}

fn write_all(f: &mut fmt::Formatter<'_>, head: &str, items: &[Expr]) -> fmt::Result {
    write!(f, "({head}")?;
    for item in items {
        write!(f, " {item}")?;
    }
    write!(f, ")")
}

/// Prints the tree as an S-expression, e.g. `1+2×3` → `(+ 1 (* 2 3))`.
/// Every operation is wrapped in brackets, so the structure is unambiguous.
impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) => write!(f, "{n}"),
            Self::Constant(c) => write!(f, "{}", c.name()),
            Self::Variable(var) => write!(f, "{}", var.name()),
            Self::Unit(u) => write!(f, "{}", u.name()),
            Self::Matrix(rows) => {
                write!(f, "(matrix")?;
                for row in rows {
                    write!(f, " ")?;
                    write_all(f, "row", row)?;
                }
                write!(f, ")")
            }
            Self::List(items) => write_all(f, "list", items),
            Self::Unary {
                op: UnaryOp::Neg,
                operand,
            } => write!(f, "(neg {operand})"),
            Self::Binary { op, left, right } => {
                let sym = match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    BinaryOp::Div => "/",
                    BinaryOp::Pow => "^",
                    BinaryOp::Polar => "∠",
                };
                write!(f, "({sym} {left} {right})")
            }
            Self::Postfix { op, operand } => {
                let sym = match op {
                    PostfixOp::Factorial => "!",
                    PostfixOp::Percent => "%",
                };
                write!(f, "({sym} {operand})")
            }
            Self::Angle { unit, value } => {
                let mark = match unit {
                    AngleMode::Degrees => "°",
                    AngleMode::Radians => "ʳ",
                    AngleMode::Gradians => "ᵍ",
                };
                write!(f, "({mark} {value})")
            }
            Self::Call { func, args } => write_all(f, func.name(), args),
            Self::Equation { left, right } => write!(f, "(= {left} {right})"),
            Self::Assign { target, value } => write!(f, "(→ {value} {})", target.name()),
            Self::Sequence(items) => write_all(f, ":", items),
        }
    }
}
