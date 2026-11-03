//! The kinds of value a calculation can produce.

mod complex;
mod matrix;

pub use complex::Complex;
pub use matrix::{MAX_DIM, Matrix};

use crate::core::engine::{evaluator::EvalError, format, settings::DisplaySettings};
use std::fmt;

/// Longest allowed list, so runaway input can't exhaust memory.
pub const MAX_LIST: usize = 1000;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    Complex(Complex),
    /// Rows × columns of real numbers, for linear algebra and vectors.
    Matrix(Matrix),
    /// A sequence of values, e.g. several roots or a data set.
    List(Vec<Value>),
}

impl Value {
    /// A real number. Complex values with no imaginary part count as real.
    pub fn as_number(&self) -> Result<f64, EvalError> {
        match self {
            Self::Number(n) => Ok(*n),
            Self::Complex(c) if c.im == 0.0 => Ok(c.re),
            _ => Err(EvalError::TypeMismatch),
        }
    }

    pub fn as_complex(&self) -> Result<Complex, EvalError> {
        match self {
            Self::Number(n) => Ok(Complex::new(*n, 0.0)),
            Self::Complex(c) => Ok(*c),
            _ => Err(EvalError::TypeMismatch),
        }
    }

    pub fn as_matrix(&self) -> Result<&Matrix, EvalError> {
        match self {
            Self::Matrix(m) => Ok(m),
            _ => Err(EvalError::TypeMismatch),
        }
    }

    /// All the real numbers inside: one for a number, every element of a
    /// matrix or list. Used by the statistics functions.
    pub fn numbers(&self) -> Result<Vec<f64>, EvalError> {
        match self {
            Self::Number(_) | Self::Complex(_) => Ok(vec![self.as_number()?]),
            Self::Matrix(m) => Ok(m.values().to_vec()),
            Self::List(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.extend(item.numbers()?);
                }
                Ok(out)
            }
        }
    }

    pub fn list(items: Vec<Value>) -> Result<Self, EvalError> {
        if items.len() > MAX_LIST {
            return Err(EvalError::LimitExceeded);
        }
        Ok(Self::List(items))
    }

    pub fn list_of_numbers(numbers: impl IntoIterator<Item = f64>) -> Result<Self, EvalError> {
        Self::list(numbers.into_iter().map(Self::Number).collect())
    }

    pub fn is_finite(&self) -> bool {
        match self {
            Self::Number(n) => n.is_finite(),
            Self::Complex(c) => c.re.is_finite() && c.im.is_finite(),
            Self::Matrix(m) => m.values().iter().all(|x| x.is_finite()),
            Self::List(items) => items.iter().all(Self::is_finite),
        }
    }

    /// Cleans up floating-point noise for display: a part much smaller than
    /// the rest of the value (below 1E-12 of it) becomes 0, and a complex
    /// number with no imaginary part becomes real. A plain number is never
    /// changed: on its own, a tiny value is not noise.
    pub fn tidy(self) -> Self {
        let snap = |x: f64, scale: f64| if x.abs() < 1e-12 * scale { 0.0 } else { x };
        match self {
            Self::Number(n) => Self::Number(n),
            Self::Complex(c) => {
                let scale = c.abs();
                let (re, im) = (snap(c.re, scale), snap(c.im, scale));
                if im == 0.0 {
                    Self::Number(re)
                } else {
                    Self::Complex(Complex::new(re, im))
                }
            }
            Self::Matrix(m) => {
                let scale = m.values().iter().fold(0.0_f64, |acc, x| acc.max(x.abs()));
                Self::Matrix(m.map(|x| snap(x, scale)))
            }
            Self::List(items) => Self::List(items.into_iter().map(Self::tidy).collect()),
        }
    }
}

/// Applies `f` to every item of a list (recursively), or to `v` itself if
/// it isn't a list. This is how `sin({0, 30})` gives `{0, 0.5}`.
pub fn broadcast(
    v: &Value,
    f: &mut dyn FnMut(&Value) -> Result<Value, EvalError>,
) -> Result<Value, EvalError> {
    match v {
        Value::List(items) => Value::list(
            items
                .iter()
                .map(|item| broadcast(item, f))
                .collect::<Result<_, _>>()?,
        ),
        other => f(other),
    }
}

/// Like [`broadcast`] for two operands: two lists pair up item by item (and
/// must be the same length); a list and a single value pair every item with
/// that value.
pub fn broadcast2(
    a: &Value,
    b: &Value,
    f: &mut dyn FnMut(&Value, &Value) -> Result<Value, EvalError>,
) -> Result<Value, EvalError> {
    match (a, b) {
        (Value::List(xs), Value::List(ys)) => {
            if xs.len() != ys.len() {
                return Err(EvalError::DimensionMismatch);
            }
            Value::list(
                xs.iter()
                    .zip(ys)
                    .map(|(x, y)| broadcast2(x, y, f))
                    .collect::<Result<_, _>>()?,
            )
        }
        (Value::List(xs), b) => Value::list(
            xs.iter()
                .map(|x| broadcast2(x, b, f))
                .collect::<Result<_, _>>()?,
        ),
        (a, Value::List(ys)) => Value::list(
            ys.iter()
                .map(|y| broadcast2(a, y, f))
                .collect::<Result<_, _>>()?,
        ),
        (a, b) => f(a, b),
    }
}

impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Self::Number(n)
    }
}

impl From<Complex> for Value {
    fn from(c: Complex) -> Self {
        Self::Complex(c)
    }
}

impl From<Matrix> for Value {
    fn from(m: Matrix) -> Self {
        Self::Matrix(m)
    }
}

/// The default display form. Use [`format::format_value`] to apply the
/// user's display settings.
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format::format_value(self, &DisplaySettings::default()))
    }
}
