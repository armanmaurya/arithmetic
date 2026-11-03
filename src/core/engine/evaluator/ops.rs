//! The arithmetic operators on every kind of value.
//!
//! - Lists work item by item: `{1, 2} + 10` is `{11, 12}`.
//! - Matrices use linear algebra: `A × B` is the matrix product, `A ÷ B`
//!   is `A × B⁻¹`, `A ^ n` repeats the product.
//! - Real numbers stay real unless complex mode is on or an operand is
//!   complex.

use super::EvalError;
use crate::core::engine::{
    context::Context,
    parser::BinaryOp,
    value::{Complex, Value, broadcast, broadcast2},
};

pub fn neg(v: &Value) -> Result<Value, EvalError> {
    broadcast(v, &mut |v| {
        Ok(match v {
            Value::Number(x) => Value::Number(-x),
            Value::Complex(z) => Value::Complex(-*z),
            Value::Matrix(m) => Value::Matrix(m.scale(-1.0)),
            Value::List(_) => unreachable!("broadcast never passes a list"),
        })
    })
}

pub fn binary(op: BinaryOp, a: &Value, b: &Value, ctx: &Context) -> Result<Value, EvalError> {
    let complex_mode = ctx.settings.complex;
    broadcast2(a, b, &mut |a, b| single(op, a, b, complex_mode))
}

/// One operation on two values that aren't lists.
fn single(op: BinaryOp, a: &Value, b: &Value, complex_mode: bool) -> Result<Value, EvalError> {
    use BinaryOp::*;
    match (a, b) {
        (Value::Matrix(x), Value::Matrix(y)) => Ok(Value::Matrix(match op {
            Add => x.add(y)?,
            Sub => x.sub(y)?,
            Mul => x.mul(y)?,
            Div => x.mul(&y.inverse()?)?,
            Pow | Polar => return Err(EvalError::TypeMismatch),
        })),
        (Value::Matrix(m), k) => {
            let k = k.as_number()?;
            Ok(Value::Matrix(match op {
                Mul => m.scale(k),
                Div if k == 0.0 => return Err(EvalError::DivisionByZero),
                Div => m.scale(1.0 / k),
                Pow => m.pow(whole_exponent(k)?)?,
                Add | Sub | Polar => return Err(EvalError::TypeMismatch),
            }))
        }
        (k, Value::Matrix(m)) => match op {
            Mul => Ok(Value::Matrix(m.scale(k.as_number()?))),
            _ => Err(EvalError::TypeMismatch),
        },
        (Value::Number(x), Value::Number(y)) => real(op, *x, *y, complex_mode),
        _ => complex(op, a.as_complex()?, b.as_complex()?),
    }
}

fn whole_exponent(k: f64) -> Result<i64, EvalError> {
    if k.fract() != 0.0 || k.abs() > 1e6 {
        return Err(EvalError::Domain);
    }
    Ok(k as i64)
}

fn real(op: BinaryOp, x: f64, y: f64, complex_mode: bool) -> Result<Value, EvalError> {
    Ok(Value::Number(match op {
        BinaryOp::Add => cancel(x + y, x, y),
        BinaryOp::Sub => cancel(x - y, x, y),
        BinaryOp::Mul => x * y,
        BinaryOp::Div if y == 0.0 => return Err(EvalError::DivisionByZero),
        BinaryOp::Div => x / y,
        BinaryOp::Pow => return power(x, y, complex_mode),
        BinaryOp::Polar => unreachable!("handled by the evaluator"),
    }))
}

/// A sum or difference far smaller than its operands is rounding noise,
/// e.g. 0.1 + 0.2 − 0.3 = 5.6E-17. Relative, so tiny values that are real
/// (like Planck's constant) are kept.
fn cancel(result: f64, x: f64, y: f64) -> f64 {
    if result.abs() < 1e-13 * x.abs().max(y.abs()) {
        0.0
    } else {
        result
    }
}

fn power(x: f64, y: f64, complex_mode: bool) -> Result<Value, EvalError> {
    if x == 0.0 && y < 0.0 {
        return Err(EvalError::DivisionByZero);
    }
    if x >= 0.0 || y.fract() == 0.0 {
        return Ok(Value::Number(x.powf(y)));
    }
    // A negative base with a fractional exponent.
    if complex_mode {
        return Ok(Value::Complex(
            Complex::new(x, 0.0).pow(Complex::new(y, 0.0))?,
        ));
    }
    // Real when the exponent is a fraction with an odd denominator:
    // (−8)^(1/3) = −2, (−8)^(2/3) = 4.
    for q in (1..100).step_by(2) {
        let p = y * q as f64;
        if (p - p.round()).abs() < 1e-9 {
            let magnitude = (-x).powf(y);
            let odd = (p.round() as i64) % 2 != 0;
            return Ok(Value::Number(if odd { -magnitude } else { magnitude }));
        }
    }
    Err(EvalError::Domain)
}

fn complex(op: BinaryOp, a: Complex, b: Complex) -> Result<Value, EvalError> {
    Ok(Value::Complex(match op {
        BinaryOp::Add => a + b,
        BinaryOp::Sub => a - b,
        BinaryOp::Mul => a * b,
        BinaryOp::Div => a.checked_div(b)?,
        BinaryOp::Pow => a.pow(b)?,
        BinaryOp::Polar => return Err(EvalError::TypeMismatch),
    }))
}
