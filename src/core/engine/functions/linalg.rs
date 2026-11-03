//! Matrices, vectors and polynomial roots.
//!
//! A vector is a one-row or one-column matrix (`[1, 2, 3]`), or a list of
//! numbers (`{1, 2, 3}`).

use super::{FunctionDef, VARIADIC, eager, finite, whole};
use crate::core::engine::{
    evaluator::EvalError,
    numeric::solve::polynomial_roots,
    value::{Complex, MAX_DIM, Matrix, Value},
};

pub(super) static DEFS: &[FunctionDef] = &[
    eager("det", &[], (1, 1), "det(A): determinant", |a, _| {
        Ok(Value::Number(finite(a[0].as_matrix()?.determinant()?)?))
    }),
    eager("inv", &[], (1, 1), "inv(A): inverse matrix", |a, _| {
        Ok(Value::Matrix(a[0].as_matrix()?.inverse()?))
    }),
    eager(
        "trn",
        &["transpose"],
        (1, 1),
        "trn(A): transpose",
        |a, _| Ok(Value::Matrix(a[0].as_matrix()?.transpose())),
    ),
    eager(
        "trace",
        &[],
        (1, 1),
        "trace(A): sum of the diagonal",
        |a, _| {
            let m = a[0].as_matrix()?;
            if m.rows() != m.cols() {
                return Err(EvalError::DimensionMismatch);
            }
            Ok(Value::Number((0..m.rows()).map(|i| m.get(i, i)).sum()))
        },
    ),
    eager(
        "iden",
        &["identity"],
        (1, 1),
        "iden(n): n×n identity matrix",
        |a, _| {
            let n = whole(a[0].as_number()?)?;
            if !(1..=MAX_DIM as i64).contains(&n) {
                return Err(EvalError::DimensionMismatch);
            }
            Ok(Value::Matrix(Matrix::identity(n as usize)?))
        },
    ),
    eager(
        "linsolve",
        &[],
        (2, 2),
        "linsolve(A, b): solves A·x = b, gives x as a list",
        |a, _| {
            let rhs = match &a[1] {
                Value::Matrix(m) => m.clone(),
                other => {
                    let b = other.numbers()?;
                    Matrix::new(b.len(), 1, b)?
                }
            };
            let x = a[0].as_matrix()?.solve(&rhs)?;
            Value::list_of_numbers(x.values().iter().copied())
        },
    ),
    eager(
        "roots",
        &[],
        (2, 4),
        "roots(a, b, …): roots of ax + b, ax² + bx + c or ax³ + bx² + cx + d",
        |a, c| {
            let coeffs: Vec<f64> = a.iter().map(Value::as_number).collect::<Result<_, _>>()?;
            roots(&coeffs, c.settings.complex)
        },
    ),
    eager(
        "item",
        &[],
        (2, 2),
        "item(list, n): the n-th value of a list or vector, from 1",
        |a, _| {
            let n = whole(a[1].as_number()?)?;
            let index = usize::try_from(n - 1).map_err(|_| EvalError::Domain)?;
            match &a[0] {
                Value::List(items) => items.get(index).cloned().ok_or(EvalError::Domain),
                v => vector(v)?
                    .get(index)
                    .map(|x| Value::Number(*x))
                    .ok_or(EvalError::Domain),
            }
        },
    ),
    // vectors
    eager("dot", &[], (2, 2), "dot(u, v): dot product", |a, _| {
        let (u, v) = (vector(&a[0])?, vector(&a[1])?);
        if u.len() != v.len() {
            return Err(EvalError::DimensionMismatch);
        }
        Ok(Value::Number(finite(
            u.iter().zip(&v).map(|(x, y)| x * y).sum(),
        )?))
    }),
    eager(
        "cross",
        &[],
        (2, 2),
        "cross(u, v): cross product of 3-vectors",
        |a, _| {
            let (u, v) = (vector(&a[0])?, vector(&a[1])?);
            let (&[u0, u1, u2], &[v0, v1, v2]) = (u.as_slice(), v.as_slice()) else {
                return Err(EvalError::DimensionMismatch);
            };
            let w = vec![u1 * v2 - u2 * v1, u2 * v0 - u0 * v2, u0 * v1 - u1 * v0];
            Ok(Value::Matrix(Matrix::new(1, 3, w)?))
        },
    ),
    eager(
        "norm",
        &[],
        (1, VARIADIC),
        "norm(v): length of a vector (Frobenius norm of a matrix)",
        |a, _| {
            let mut sum = 0.0;
            for v in a {
                sum += v.numbers()?.iter().map(|x| x * x).sum::<f64>();
            }
            Ok(Value::Number(finite(sum.sqrt())?))
        },
    ),
    eager(
        "unit",
        &[],
        (1, 1),
        "unit(v): the vector scaled to length 1",
        |a, _| {
            let v = vector(&a[0])?;
            let length = v.iter().map(|x| x * x).sum::<f64>().sqrt();
            if length == 0.0 {
                return Err(EvalError::DivisionByZero);
            }
            let n = v.len();
            Ok(Value::Matrix(Matrix::new(
                1,
                n,
                v.into_iter().map(|x| x / length).collect(),
            )?))
        },
    ),
    eager(
        "angle",
        &[],
        (2, 2),
        "angle(u, v): angle between two vectors",
        |a, c| {
            let (u, v) = (vector(&a[0])?, vector(&a[1])?);
            if u.len() != v.len() {
                return Err(EvalError::DimensionMismatch);
            }
            let dot: f64 = u.iter().zip(&v).map(|(x, y)| x * y).sum();
            let lengths = u.iter().map(|x| x * x).sum::<f64>().sqrt()
                * v.iter().map(|x| x * x).sum::<f64>().sqrt();
            if lengths == 0.0 {
                return Err(EvalError::Domain);
            }
            let cos = (dot / lengths).clamp(-1.0, 1.0);
            Ok(Value::Number(c.settings.angle.from_radians(cos.acos())))
        },
    ),
];

fn vector(v: &Value) -> Result<Vec<f64>, EvalError> {
    match v {
        Value::Matrix(m) if m.rows() == 1 || m.cols() == 1 => Ok(m.values().to_vec()),
        Value::Matrix(_) => Err(EvalError::DimensionMismatch),
        Value::List(_) => v.numbers(),
        _ => Err(EvalError::TypeMismatch),
    }
}

/// Polynomial roots as a list. In complex mode, roots that aren't real are
/// included as complex numbers instead of failing.
fn roots(coeffs: &[f64], complex: bool) -> Result<Value, EvalError> {
    let trimmed = match coeffs.iter().position(|&c| c != 0.0) {
        Some(i) => &coeffs[i..],
        None => coeffs,
    };
    let numbers = |xs: Vec<f64>| Value::list_of_numbers(xs);
    match (polynomial_roots(trimmed), trimmed) {
        (Err(EvalError::NoRealRoots), &[a, b, c]) if complex => Value::list(quadratic(a, b, c)),
        (Ok(real), &[a, b, c, _]) if complex && real.len() == 1 => {
            // One real root r: divide it out, leaving a quadratic.
            let r = real[0];
            let (qb, qc) = (b + a * r, c + (b + a * r) * r);
            let mut all = vec![Value::Number(r)];
            all.extend(quadratic(a, qb, qc).into_iter().filter(|v| match v {
                Value::Number(x) => (x - r).abs() > 1e-9 * r.abs().max(1.0),
                _ => true,
            }));
            Value::list(all)
        }
        (result, _) => numbers(result?),
    }
}

/// Both roots of `ax² + bx + c`, as a complex pair when they aren't real.
fn quadratic(a: f64, b: f64, c: f64) -> Vec<Value> {
    let disc = b * b - 4.0 * a * c;
    if disc >= 0.0 {
        let s = disc.sqrt();
        let mut rs = vec![(-b - s) / (2.0 * a), (-b + s) / (2.0 * a)];
        rs.sort_by(f64::total_cmp);
        rs.dedup();
        return rs.into_iter().map(Value::Number).collect();
    }
    let re = -b / (2.0 * a);
    let im = (-disc).sqrt() / (2.0 * a).abs();
    vec![
        Value::Complex(Complex::new(re, im)),
        Value::Complex(Complex::new(re, -im)),
    ]
}
