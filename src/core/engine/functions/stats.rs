//! Statistics, regression and probability distributions.
//!
//! Data functions take any mix of numbers, lists and matrices:
//! `mean(1, 2, 3)`, `mean({1, 2, 3})` and `mean([1, 2; 3, 4])` all work.
//! Regressions take the x data and the y data as two lists.

use super::{FunctionDef, VARIADIC, arg_or, eager, finite, real, whole};
use crate::core::engine::{
    evaluator::EvalError,
    numeric::special,
    value::{Matrix, Value},
};

pub(super) static DEFS: &[FunctionDef] = &[
    eager("sum", &[], (1, VARIADIC), "sum(data): total", |a, _| {
        number(data(a)?.iter().sum())
    }),
    eager(
        "prod",
        &[],
        (1, VARIADIC),
        "prod(data): product of all values",
        |a, _| number(data(a)?.iter().product()),
    ),
    eager(
        "rep",
        &[],
        (2, 2),
        "rep(x, n): a list of n copies of x, e.g. data with a frequency",
        |a, _| {
            let n = whole(a[1].as_number()?)?;
            if n < 0 {
                return Err(EvalError::Domain);
            }
            if n as usize > crate::core::engine::value::MAX_LIST {
                return Err(EvalError::LimitExceeded);
            }
            Value::list(vec![a[0].clone(); n as usize])
        },
    ),
    eager(
        "count",
        &["len"],
        (1, VARIADIC),
        "count(data): how many values",
        |a, _| Ok(Value::Number(data(a)?.len() as f64)),
    ),
    eager(
        "mean",
        &["avg"],
        (1, VARIADIC),
        "mean(data): arithmetic mean",
        |a, _| number(mean(&nonempty(a)?)),
    ),
    eager(
        "median",
        &[],
        (1, VARIADIC),
        "median(data): middle value",
        |a, _| {
            let mut xs = nonempty(a)?;
            xs.sort_by(f64::total_cmp);
            let mid = xs.len() / 2;
            number(if xs.len() % 2 == 0 {
                (xs[mid - 1] + xs[mid]) / 2.0
            } else {
                xs[mid]
            })
        },
    ),
    eager(
        "var",
        &[],
        (1, VARIADIC),
        "var(data): sample variance",
        |a, _| number(variance(&data(a)?, 1)?),
    ),
    eager(
        "varp",
        &[],
        (1, VARIADIC),
        "varp(data): population variance",
        |a, _| number(variance(&data(a)?, 0)?),
    ),
    eager(
        "sd",
        &["stdev"],
        (1, VARIADIC),
        "sd(data): sample standard deviation",
        |a, _| number(variance(&data(a)?, 1)?.sqrt()),
    ),
    eager(
        "sdp",
        &["stdevp"],
        (1, VARIADIC),
        "sdp(data): population standard deviation",
        |a, _| number(variance(&data(a)?, 0)?.sqrt()),
    ),
    eager(
        "min",
        &[],
        (1, VARIADIC),
        "min(data): smallest value",
        |a, _| number(nonempty(a)?.into_iter().fold(f64::INFINITY, f64::min)),
    ),
    eager(
        "max",
        &[],
        (1, VARIADIC),
        "max(data): largest value",
        |a, _| number(nonempty(a)?.into_iter().fold(f64::NEG_INFINITY, f64::max)),
    ),
    eager(
        "sort",
        &[],
        (1, VARIADIC),
        "sort(data): values in ascending order",
        |a, _| {
            let mut xs = data(a)?;
            xs.sort_by(f64::total_cmp);
            Value::list_of_numbers(xs)
        },
    ),
    // regressions
    eager(
        "linreg",
        &[],
        (2, 2),
        "linreg(xs, ys): fits y = a + bx, gives {a, b}",
        |a, _| {
            let (xs, ys) = pairs(a, Ok, Ok)?;
            let (a0, b) = line(&xs, &ys)?;
            Value::list_of_numbers([a0, b])
        },
    ),
    eager(
        "quadreg",
        &[],
        (2, 2),
        "quadreg(xs, ys): fits y = a + bx + cx², gives {a, b, c}",
        |a, _| {
            let (xs, ys) = pairs(a, Ok, Ok)?;
            if xs.len() < 3 {
                return Err(EvalError::Domain);
            }
            // The normal equations: Σ xⁱ⁺ʲ · coefficient = Σ xⁱ y
            let power_sum = |p: i32| xs.iter().map(|x| x.powi(p)).sum::<f64>();
            let mut lhs = Vec::with_capacity(9);
            for i in 0..3 {
                for j in 0..3 {
                    lhs.push(power_sum(i + j));
                }
            }
            let rhs: Vec<f64> = (0..3)
                .map(|i| xs.iter().zip(&ys).map(|(x, y)| x.powi(i) * y).sum())
                .collect();
            let solution = Matrix::new(3, 3, lhs)?.solve(&Matrix::new(3, 1, rhs)?)?;
            Value::list_of_numbers(solution.values().iter().copied())
        },
    ),
    eager(
        "logreg",
        &[],
        (2, 2),
        "logreg(xs, ys): fits y = a + b·ln x, gives {a, b}",
        |a, _| {
            let (xs, ys) = pairs(a, positive_ln, Ok)?;
            let (a0, b) = line(&xs, &ys)?;
            Value::list_of_numbers([a0, b])
        },
    ),
    eager(
        "expreg",
        &[],
        (2, 2),
        "expreg(xs, ys): fits y = a·e^(bx), gives {a, b}",
        |a, _| {
            let (xs, ys) = pairs(a, Ok, positive_ln)?;
            let (ln_a, b) = line(&xs, &ys)?;
            Value::list_of_numbers([ln_a.exp(), b])
        },
    ),
    eager(
        "powreg",
        &[],
        (2, 2),
        "powreg(xs, ys): fits y = a·x^b, gives {a, b}",
        |a, _| {
            let (xs, ys) = pairs(a, positive_ln, positive_ln)?;
            let (ln_a, b) = line(&xs, &ys)?;
            Value::list_of_numbers([ln_a.exp(), b])
        },
    ),
    eager(
        "corr",
        &[],
        (2, 2),
        "corr(xs, ys): correlation coefficient r",
        |a, _| {
            let (xs, ys) = pairs(a, Ok, Ok)?;
            let (sxx, sxy, syy) = sums(&xs, &ys);
            if sxx == 0.0 || syy == 0.0 {
                return Err(EvalError::Domain);
            }
            number(sxy / (sxx * syy).sqrt())
        },
    ),
    // distributions
    eager(
        "normpdf",
        &[],
        (1, 3),
        "normpdf(x, μ, σ): normal density (μ = 0, σ = 1 by default)",
        |a, _| {
            let (mu, sigma) = mu_sigma(a, 1)?;
            real(&a[0], |x| Ok(special::normal_pdf((x - mu) / sigma) / sigma))
        },
    ),
    eager(
        "normcdf",
        &[],
        (2, 4),
        "normcdf(lower, upper, μ, σ): P(lower ≤ X ≤ upper)",
        |a, _| {
            let (lower, upper) = (a[0].as_number()?, a[1].as_number()?);
            let (mu, sigma) = mu_sigma(a, 2)?;
            let z = |x: f64| (x - mu) / sigma;
            number(special::normal_cdf(z(upper)) - special::normal_cdf(z(lower)))
        },
    ),
    eager(
        "invnorm",
        &[],
        (1, 3),
        "invnorm(p, μ, σ): the x with P(X ≤ x) = p",
        |a, _| {
            let (mu, sigma) = mu_sigma(a, 1)?;
            real(&a[0], |p| {
                Ok(mu + sigma * special::inverse_normal(p).ok_or(EvalError::Domain)?)
            })
        },
    ),
    eager(
        "binompdf",
        &[],
        (3, 3),
        "binompdf(n, p, k): P(X = k), binomial",
        |a, _| {
            let (n, p) = n_p(a)?;
            real(&a[2], |k| binomial_pmf(n, p, k))
        },
    ),
    eager(
        "binomcdf",
        &[],
        (3, 3),
        "binomcdf(n, p, k): P(X ≤ k), binomial",
        |a, _| {
            let (n, p) = n_p(a)?;
            real(&a[2], |k| {
                let k = k.floor().min(n as f64);
                if k < 0.0 {
                    return Ok(0.0);
                }
                let mut total = 0.0;
                for j in 0..=(k as u64) {
                    total += binomial_pmf(n, p, j as f64)?;
                }
                Ok(total.min(1.0))
            })
        },
    ),
    eager(
        "poissonpdf",
        &[],
        (2, 2),
        "poissonpdf(λ, k): P(X = k), Poisson",
        |a, _| {
            let lambda = rate(a)?;
            real(&a[1], |k| poisson_pmf(lambda, k))
        },
    ),
    eager(
        "poissoncdf",
        &[],
        (2, 2),
        "poissoncdf(λ, k): P(X ≤ k), Poisson",
        |a, _| {
            let lambda = rate(a)?;
            real(&a[1], |k| {
                let k = k.floor();
                if k < 0.0 {
                    return Ok(0.0);
                }
                if k > 1e6 {
                    return Err(EvalError::LimitExceeded);
                }
                let mut total = 0.0;
                for j in 0..=(k as u64) {
                    total += poisson_pmf(lambda, j as f64)?;
                }
                Ok(total.min(1.0))
            })
        },
    ),
];

fn number(x: f64) -> Result<Value, EvalError> {
    Ok(Value::Number(finite(x)?))
}

/// Every number in every argument.
fn data(args: &[Value]) -> Result<Vec<f64>, EvalError> {
    let mut out = Vec::new();
    for v in args {
        out.extend(v.numbers()?);
    }
    Ok(out)
}

fn nonempty(args: &[Value]) -> Result<Vec<f64>, EvalError> {
    let xs = data(args)?;
    if xs.is_empty() {
        return Err(EvalError::Domain);
    }
    Ok(xs)
}

fn mean(xs: &[f64]) -> f64 {
    xs.iter().sum::<f64>() / xs.len() as f64
}

/// Variance, dividing by `n − correction` (1 for a sample, 0 for a whole
/// population). Two passes, which is more accurate than Σx² − (Σx)²/n.
fn variance(xs: &[f64], correction: usize) -> Result<f64, EvalError> {
    if xs.len() <= correction || xs.is_empty() {
        return Err(EvalError::Domain);
    }
    let m = mean(xs);
    Ok(xs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (xs.len() - correction) as f64)
}

/// The x and y data of a regression, each passed through a transform (for
/// fitting curves as straight lines, e.g. ln y against x).
fn pairs(
    args: &[Value],
    fx: impl Fn(f64) -> Result<f64, EvalError>,
    fy: impl Fn(f64) -> Result<f64, EvalError>,
) -> Result<(Vec<f64>, Vec<f64>), EvalError> {
    let xs = args[0].numbers()?;
    let ys = args[1].numbers()?;
    if xs.len() != ys.len() {
        return Err(EvalError::DimensionMismatch);
    }
    if xs.len() < 2 {
        return Err(EvalError::Domain);
    }
    Ok((
        xs.into_iter().map(fx).collect::<Result<_, _>>()?,
        ys.into_iter().map(fy).collect::<Result<_, _>>()?,
    ))
}

fn positive_ln(x: f64) -> Result<f64, EvalError> {
    if x > 0.0 {
        Ok(x.ln())
    } else {
        Err(EvalError::Domain)
    }
}

/// Sums of squared and cross deviations: (Sxx, Sxy, Syy).
fn sums(xs: &[f64], ys: &[f64]) -> (f64, f64, f64) {
    let (mx, my) = (mean(xs), mean(ys));
    xs.iter()
        .zip(ys)
        .fold((0.0, 0.0, 0.0), |(sxx, sxy, syy), (x, y)| {
            let (dx, dy) = (x - mx, y - my);
            (sxx + dx * dx, sxy + dx * dy, syy + dy * dy)
        })
}

/// Least-squares line y = a + bx.
fn line(xs: &[f64], ys: &[f64]) -> Result<(f64, f64), EvalError> {
    let (sxx, sxy, _) = sums(xs, ys);
    if sxx == 0.0 {
        return Err(EvalError::Domain);
    }
    let b = sxy / sxx;
    Ok((finite(mean(ys) - b * mean(xs))?, finite(b)?))
}

/// μ and σ from the arguments starting at `first`.
fn mu_sigma(args: &[Value], first: usize) -> Result<(f64, f64), EvalError> {
    let mu = arg_or(args, first, 0.0)?;
    let sigma = arg_or(args, first + 1, 1.0)?;
    if sigma <= 0.0 {
        return Err(EvalError::Domain);
    }
    Ok((mu, sigma))
}

fn n_p(args: &[Value]) -> Result<(u64, f64), EvalError> {
    let n = whole(args[0].as_number()?)?;
    let p = args[1].as_number()?;
    if n < 0 || !(0.0..=1.0).contains(&p) {
        return Err(EvalError::Domain);
    }
    if n > 1_000_000 {
        return Err(EvalError::LimitExceeded);
    }
    Ok((n as u64, p))
}

fn rate(args: &[Value]) -> Result<f64, EvalError> {
    let lambda = args[0].as_number()?;
    if lambda <= 0.0 {
        return Err(EvalError::Domain);
    }
    Ok(lambda)
}

fn binomial_pmf(n: u64, p: f64, k: f64) -> Result<f64, EvalError> {
    let k = whole(k)?;
    if k < 0 || k as u64 > n {
        return Ok(0.0);
    }
    let (n, k) = (n as f64, k as f64);
    if p == 0.0 || p == 1.0 {
        let certain = if p == 0.0 { 0.0 } else { n };
        return Ok(if k == certain { 1.0 } else { 0.0 });
    }
    let ln_choose =
        special::ln_gamma(n + 1.0) - special::ln_gamma(k + 1.0) - special::ln_gamma(n - k + 1.0);
    finite((ln_choose + k * p.ln() + (n - k) * (1.0 - p).ln()).exp())
}

fn poisson_pmf(lambda: f64, k: f64) -> Result<f64, EvalError> {
    let k = whole(k)?;
    if k < 0 {
        return Ok(0.0);
    }
    let k = k as f64;
    finite((k * lambda.ln() - lambda - special::ln_gamma(k + 1.0)).exp())
}
