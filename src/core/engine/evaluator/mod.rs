//! The evaluator: walks the tree from the parser and computes a value.
//! Operators live in `ops`, functions in the `functions` registry; this
//! file only connects the tree to them.

mod error;
pub mod ops;

#[cfg(test)]
mod tests;

pub use error::EvalError;

use crate::core::engine::{
    context::Context,
    functions::Body,
    lexer::Var,
    parser::{BinaryOp, Expr, PostfixOp, UnaryOp},
    settings::{AngleMode, PercentStyle},
    value::{Complex, Matrix, Value, broadcast},
};

pub fn evaluate(expr: &Expr, ctx: &mut Context) -> Result<Value, EvalError> {
    let value = match expr {
        Expr::Number(n) => Value::Number(*n),
        Expr::Constant(c) => {
            let z = c.def().value;
            if z.im == 0.0 {
                Value::Number(z.re)
            } else {
                Value::Complex(z)
            }
        }
        Expr::Variable(v) => ctx.get(*v),
        Expr::Unit(_) => return Err(EvalError::UnitMismatch),
        Expr::Matrix(rows) => {
            let cols = rows.first().map_or(0, Vec::len);
            let mut data = Vec::with_capacity(rows.len() * cols);
            for cell in rows.iter().flatten() {
                data.push(evaluate(cell, ctx)?.as_number()?);
            }
            Value::Matrix(Matrix::new(rows.len(), cols, data)?)
        }
        // A list inside a list literal is spliced in: {1, rep(2, 3)} is
        // {1, 2, 2, 2}.
        Expr::List(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                match evaluate(item, ctx)? {
                    Value::List(inner) => out.extend(inner),
                    v => out.push(v),
                }
            }
            Value::list(out)?
        }
        Expr::Unary {
            op: UnaryOp::Neg,
            operand,
        } => ops::neg(&evaluate(operand, ctx)?)?,
        Expr::Binary { op, left, right } => binary(*op, left, right, ctx)?,
        Expr::Postfix { op, operand } => {
            let v = evaluate(operand, ctx)?;
            match op {
                PostfixOp::Percent => ops::binary(BinaryOp::Div, &v, &Value::Number(100.0), ctx)?,
                PostfixOp::Factorial => {
                    broadcast(&v, &mut |x| Ok(Value::Number(factorial(x.as_number()?)?)))?
                }
            }
        }
        Expr::Angle { unit, value } => {
            let (from, to) = (*unit, ctx.settings.angle);
            broadcast(&evaluate(value, ctx)?, &mut |x| {
                let x = x.as_number()?;
                Ok(Value::Number(match (from, to) {
                    _ if from == to => x,
                    (AngleMode::Degrees, _) => to.from_degrees(x),
                    _ => to.from_radians(from.to_radians(x)),
                }))
            })?
        }
        Expr::Call { func, args } => {
            let def = func.def();
            if !(def.min_args..=def.max_args).contains(&args.len()) {
                return Err(EvalError::TypeMismatch);
            }
            match def.body {
                Body::Eager(run) => {
                    let values = args
                        .iter()
                        .map(|a| evaluate(a, ctx))
                        .collect::<Result<Vec<_>, _>>()?;
                    run(&values, ctx)?
                }
                Body::Lazy(run) => run(args, ctx)?,
            }
        }
        Expr::Equation { .. } => return Err(EvalError::MisplacedEquation),
        Expr::Assign { target, value } => {
            let v = evaluate(value, ctx)?.tidy();
            ctx.set(*target, v.clone());
            v
        }
        Expr::Sequence(items) => {
            let mut last = Value::Number(0.0);
            for item in items {
                last = evaluate(item, ctx)?.tidy();
                ctx.set(Var::Ans, last.clone());
            }
            last
        }
    };
    // Checked at every node, so an overflow or NaN stops evaluation at once.
    if value.is_finite() {
        Ok(value)
    } else {
        Err(EvalError::InvalidResult)
    }
}

fn binary(op: BinaryOp, left: &Expr, right: &Expr, ctx: &mut Context) -> Result<Value, EvalError> {
    let a = evaluate(left, ctx)?;
    // `a ± b%` as "a plus/minus b percent of a", if the settings say so.
    if let (BinaryOp::Add | BinaryOp::Sub, PercentStyle::OfLeftOperand) = (op, ctx.settings.percent)
        && let Expr::Postfix {
            op: PostfixOp::Percent,
            operand,
        } = right
    {
        let rate = ops::binary(
            BinaryOp::Div,
            &evaluate(operand, ctx)?,
            &Value::Number(100.0),
            ctx,
        )?;
        let part = ops::binary(BinaryOp::Mul, &a, &rate, ctx)?;
        return ops::binary(op, &a, &part, ctx);
    }
    let b = evaluate(right, ctx)?;
    if op == BinaryOp::Polar {
        let angle = ctx.settings.angle;
        return crate::core::engine::value::broadcast2(&a, &b, &mut |r, theta| {
            let z = Complex::from_polar(r.as_number()?, angle.to_radians(theta.as_number()?));
            Ok(Value::Complex(z))
        });
    }
    ops::binary(op, &a, &b, ctx)
}

/// Whole numbers 0 to 170 only (171! overflows an f64).
fn factorial(x: f64) -> Result<f64, EvalError> {
    // Small tolerance so that e.g. (0.1+0.2)×10 = 3.0000000000000004 still works.
    if x < 0.0 || (x - x.round()).abs() > 1e-9 {
        return Err(EvalError::Domain);
    }
    let n = x.round() as u64;
    if n > 170 {
        return Err(EvalError::InvalidResult);
    }
    Ok((2..=n).map(|k| k as f64).product())
}
