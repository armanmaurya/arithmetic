//! Functions that evaluate an expression in `X` many times: solving,
//! calculus, sums and tables. Plus `convert`, whose units aren't values.
//!
//! `X` is the bound variable. Its value is restored afterwards, except by
//! `solve`, which stores the root in `X` the way calculators do.

use super::{FunctionDef, finite, lazy, whole};
use crate::core::engine::{
    context::Context,
    evaluator::{self, EvalError, ops},
    lexer::Var,
    numeric::{calculus, solve::find_root},
    parser::{BinaryOp, Expr},
    units,
    value::{MAX_LIST, Value, broadcast},
};

pub(super) static DEFS: &[FunctionDef] = &[
    lazy(
        "solve",
        &[],
        (1, 2),
        "solve(f = g, guess): an X where f = g (or f = 0), near guess",
        solve,
    ),
    lazy(
        "integrate",
        &["∫"],
        (3, 3),
        "integrate(f, a, b): ∫ f dX from a to b",
        |a, c| {
            let (lo, hi) = (number(&a[1], c)?, number(&a[2], c)?);
            keeping_x(c, |c| {
                let mut curve = Curve::new(&a[0], None, c);
                let result = calculus::integrate(&mut |x| curve.at(x), lo, hi);
                curve.finish(result).map(Value::Number)
            })
        },
    ),
    lazy(
        "derivative",
        &["d/dx"],
        (2, 2),
        "derivative(f, a): dF/dX at X = a",
        |a, c| {
            let at = number(&a[1], c)?;
            keeping_x(c, |c| {
                let mut curve = Curve::new(&a[0], None, c);
                let result = calculus::derivative(&mut |x| curve.at(x), at);
                curve.finish(result).map(Value::Number)
            })
        },
    ),
    lazy(
        "sigma",
        &["Σ"],
        (3, 3),
        "sigma(f, a, b): f summed for X = a, a+1, …, b",
        |a, c| series(a, c, Value::Number(0.0), BinaryOp::Add),
    ),
    lazy(
        "product",
        &["Π"],
        (3, 3),
        "product(f, a, b): f multiplied for X = a, a+1, …, b",
        |a, c| series(a, c, Value::Number(1.0), BinaryOp::Mul),
    ),
    lazy(
        "table",
        &[],
        (3, 4),
        "table(f, start, end, step): {X, f} pairs",
        |a, c| {
            let (start, end) = (number(&a[1], c)?, number(&a[2], c)?);
            let step = match a.get(3) {
                Some(e) => number(e, c)?,
                None => 1.0,
            };
            if step == 0.0 || (end - start) * step < 0.0 {
                return Err(EvalError::Domain);
            }
            let count = ((end - start) / step + 1e-9).floor() + 1.0;
            if count > MAX_LIST as f64 {
                return Err(EvalError::LimitExceeded);
            }
            keeping_x(c, |c| {
                let mut rows = Vec::with_capacity(count as usize);
                for i in 0..count as usize {
                    let x = start + step * i as f64;
                    let y = at_x(&a[0], x, c)?;
                    rows.push(Value::list(vec![Value::Number(x), y])?);
                }
                Value::list(rows)
            })
        },
    ),
    lazy(
        "convert",
        &[],
        (3, 3),
        "convert(x, _from, _to): unit conversion, e.g. convert(5, _km, _mi)",
        |a, c| {
            let (Expr::Unit(from), Expr::Unit(to)) = (&a[1], &a[2]) else {
                return Err(EvalError::UnitMismatch);
            };
            let value = evaluator::evaluate(&a[0], c)?;
            broadcast(&value, &mut |v| {
                let x =
                    units::convert(v.as_number()?, *from, *to).ok_or(EvalError::UnitMismatch)?;
                Ok(Value::Number(finite(x)?))
            })
        },
    ),
];

fn number(expr: &Expr, ctx: &mut Context) -> Result<f64, EvalError> {
    evaluator::evaluate(expr, ctx)?.as_number()
}

/// Runs `f`, then puts `X` back the way it was.
fn keeping_x<T>(ctx: &mut Context, f: impl FnOnce(&mut Context) -> T) -> T {
    let saved = ctx.get(Var::X);
    let result = f(ctx);
    ctx.set(Var::X, saved);
    result
}

/// `body` evaluated with `X = x`.
fn at_x(body: &Expr, x: f64, ctx: &mut Context) -> Result<Value, EvalError> {
    ctx.spend(1)?;
    ctx.set(Var::X, Value::Number(x));
    evaluator::evaluate(body, ctx)
}

/// An expression in `X` as a real function for the numeric methods: `left`,
/// or `left − right` for an equation. Points where it is undefined give
/// `None`. Running out of work budget is remembered, so it can be reported
/// instead of a confusing "no solution".
struct Curve<'a> {
    left: &'a Expr,
    right: Option<&'a Expr>,
    ctx: &'a mut Context,
    limit: Option<EvalError>,
}

impl<'a> Curve<'a> {
    fn new(left: &'a Expr, right: Option<&'a Expr>, ctx: &'a mut Context) -> Self {
        Self {
            left,
            right,
            ctx,
            limit: None,
        }
    }

    fn at(&mut self, x: f64) -> Option<f64> {
        if self.limit.is_some() {
            return None;
        }
        let mut side = |expr: &Expr| match at_x(expr, x, self.ctx).and_then(|v| v.as_number()) {
            Ok(y) => Some(y),
            Err(EvalError::LimitExceeded) => {
                self.limit = Some(EvalError::LimitExceeded);
                None
            }
            Err(_) => None,
        };
        let left = side(self.left)?;
        match self.right {
            Some(right) => Some(left - side(right)?),
            None => Some(left),
        }
    }

    fn finish<T>(self, result: Result<T, EvalError>) -> Result<T, EvalError> {
        match self.limit {
            Some(e) => Err(e),
            None => result,
        }
    }
}

fn solve(args: &[Expr], ctx: &mut Context) -> Result<Value, EvalError> {
    let (left, right) = match &args[0] {
        Expr::Equation { left, right } => (&**left, Some(&**right)),
        other => (other, None),
    };
    let guess = match args.get(1) {
        Some(e) => number(e, ctx)?,
        None => ctx.get(Var::X).as_number().unwrap_or(0.0),
    };
    let saved = ctx.get(Var::X);
    let mut curve = Curve::new(left, right, ctx);
    let result = find_root(|x| curve.at(x), guess);
    let result = curve.finish(result);
    match result {
        Ok(root) => {
            let root = if root.abs() < 1e-12 { 0.0 } else { root };
            ctx.set(Var::X, Value::Number(root));
            Ok(Value::Number(root))
        }
        Err(e) => {
            ctx.set(Var::X, saved);
            Err(e)
        }
    }
}

/// Σ and Π: combines `f(X)` for every whole `X` from `a` to `b`.
fn series(
    args: &[Expr],
    ctx: &mut Context,
    empty: Value,
    op: BinaryOp,
) -> Result<Value, EvalError> {
    let (a, b) = (
        whole(number(&args[1], ctx)?)?,
        whole(number(&args[2], ctx)?)?,
    );
    if b - a > 1_000_000 {
        return Err(EvalError::LimitExceeded);
    }
    keeping_x(ctx, |ctx| {
        let mut total = empty;
        for x in a..=b {
            let term = at_x(&args[0], x as f64, ctx)?;
            total = ops::binary(op, &total, &term, ctx)?;
        }
        Ok(total)
    })
}
