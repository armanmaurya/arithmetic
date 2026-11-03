use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalError {
    DivisionByZero,
    /// Square root of a negative number, outside complex mode.
    NegativeRoot,
    /// An argument outside a function's domain, e.g. `ln(0)`, `tan(90°)`, `2.5!`.
    Domain,
    /// The result overflowed to infinity or is not a number, e.g. `10^400`.
    InvalidResult,
    /// A value of the wrong kind, e.g. `sin([1, 2])` or `det(5)`.
    TypeMismatch,
    /// Sizes that don't fit, e.g. adding a 2×2 and a 3×3 matrix.
    DimensionMismatch,
    SingularMatrix,
    NoRealRoots,
    /// A solver couldn't find an answer.
    NoSolution,
    /// `=` outside a function that takes an equation, e.g. `2 = 3`.
    MisplacedEquation,
    /// The calculation needs more work or memory than allowed.
    LimitExceeded,
    /// A unit outside `convert`, or units of different kinds, e.g. metres
    /// to seconds.
    UnitMismatch,
}

/// These are the messages the calculator screen shows.
impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::DivisionByZero => "Can't divide by 0",
            Self::NegativeRoot => "Keep it real",
            Self::Domain => "Domain error",
            Self::InvalidResult => "Value error",
            Self::TypeMismatch => "Type error",
            Self::DimensionMismatch => "Dimension error",
            Self::SingularMatrix => "Singular matrix",
            Self::NoRealRoots => "No real roots",
            Self::NoSolution => "No solution found",
            Self::MisplacedEquation => "Format error",
            Self::LimitExceeded => "Limit exceeded",
            Self::UnitMismatch => "Unit error",
        })
    }
}

impl std::error::Error for EvalError {}
