//! Numeric solvers. They work on plain numbers and closures and know nothing
//! about the parser or the tree.

use crate::core::engine::evaluator::EvalError;

/// Real roots of a polynomial, highest power first: `[a, b, c]` is
/// `ax² + bx + c`. Up to cubics. Returned in ascending order, each once.
pub fn polynomial_roots(coefficients: &[f64]) -> Result<Vec<f64>, EvalError> {
    let first_nonzero = coefficients.iter().position(|&c| c != 0.0);
    let coeffs = match first_nonzero {
        Some(i) => &coefficients[i..],
        None => return Err(EvalError::Domain), // 0 = 0: every x is a root
    };

    let mut roots = match *coeffs {
        [_] => return Err(EvalError::NoSolution), // e.g. 5 = 0
        [a, b] => vec![-b / a],
        [a, b, c] => quadratic(a, b, c)?,
        [a, b, c, d] => cubic(a, b, c, d),
        _ => return Err(EvalError::Domain),
    };

    // Polish each root with a few Newton steps on the original polynomial.
    for root in &mut roots {
        for _ in 0..3 {
            let (value, slope) = eval_with_slope(coeffs, *root);
            if slope == 0.0 {
                break;
            }
            let next = *root - value / slope;
            if next.is_finite() {
                *root = next;
            }
        }
        if root.abs() < 1e-12 {
            *root = 0.0;
        }
    }

    roots.sort_by(f64::total_cmp);
    roots.dedup_by(|a, b| (*a - *b).abs() <= 1e-9 * a.abs().max(1.0));
    if roots.iter().all(|r| r.is_finite()) {
        Ok(roots)
    } else {
        Err(EvalError::InvalidResult)
    }
}

/// The value and slope of the polynomial at `x` (Horner's method).
fn eval_with_slope(coeffs: &[f64], x: f64) -> (f64, f64) {
    let (mut value, mut slope) = (0.0, 0.0);
    for &c in coeffs {
        slope = slope * x + value;
        value = value * x + c;
    }
    (value, slope)
}

fn quadratic(a: f64, b: f64, c: f64) -> Result<Vec<f64>, EvalError> {
    let disc = b * b - 4.0 * a * c;
    let noise = 1e-12 * (b * b).max((4.0 * a * c).abs());
    if disc < -noise {
        return Err(EvalError::NoRealRoots);
    }
    if disc.abs() <= noise {
        return Ok(vec![-b / (2.0 * a)]);
    }
    // The stable form: avoids subtracting two nearly equal numbers.
    let q = -0.5 * (b + b.signum() * disc.sqrt());
    if q == 0.0 {
        return Ok(vec![0.0]);
    }
    Ok(vec![q / a, c / q])
}

/// Real roots of `ax³ + bx² + cx + d` (Cardano / trigonometric method).
fn cubic(a: f64, b: f64, c: f64, d: f64) -> Vec<f64> {
    let (b, c, d) = (b / a, c / a, d / a);
    // Substitute x = t − b/3 to get t³ + pt + q = 0.
    let shift = b / 3.0;
    let p = c - b * b / 3.0;
    let q = 2.0 * b * b * b / 27.0 - b * c / 3.0 + d;
    let disc = (q / 2.0).powi(2) + (p / 3.0).powi(3);
    let noise = 1e-12 * (q * q).max(p.abs().powi(3)).max(1e-300);

    let ts = if disc > noise {
        let s = disc.sqrt();
        vec![(-q / 2.0 + s).cbrt() + (-q / 2.0 - s).cbrt()]
    } else if disc.abs() <= noise {
        if p.abs() <= 1e-12 {
            vec![0.0]
        } else {
            vec![3.0 * q / p, -3.0 * q / (2.0 * p)]
        }
    } else {
        let r = 2.0 * (-p / 3.0).sqrt();
        let phi = ((3.0 * q / (2.0 * p)) * (-3.0 / p).sqrt())
            .clamp(-1.0, 1.0)
            .acos()
            / 3.0;
        (0..3)
            .map(|k| r * (phi - 2.0 * std::f64::consts::PI * k as f64 / 3.0).cos())
            .collect()
    };
    ts.into_iter().map(|t| t - shift).collect()
}

/// Finds an `x` near `guess` where `f(x) = 0`. `f` returns `None` where it is
/// undefined (e.g. `ln` of a negative number).
///
/// Tries Newton's method first (fast), then searches outward from `guess` for
/// a sign change and narrows it down by bisection (slow but reliable).
pub fn find_root(mut f: impl FnMut(f64) -> Option<f64>, guess: f64) -> Result<f64, EvalError> {
    const GOOD_ENOUGH: f64 = 1e-9;
    let is_root = |f: &mut dyn FnMut(f64) -> Option<f64>, x: f64| {
        f(x).is_some_and(|y| y.abs() <= GOOD_ENOUGH)
    };

    // 1. Newton's method, with a numerical derivative.
    let mut x = guess;
    for _ in 0..100 {
        let Some(fx) = f(x) else { break };
        if fx == 0.0 {
            return Ok(x);
        }
        let h = 1e-6 * x.abs().max(1.0);
        let (Some(ahead), Some(behind)) = (f(x + h), f(x - h)) else {
            break;
        };
        let slope = (ahead - behind) / (2.0 * h);
        let next = x - fx / slope;
        if !next.is_finite() {
            break;
        }
        if (next - x).abs() <= 1e-13 * next.abs().max(1.0) {
            if is_root(&mut f, next) {
                return Ok(next);
            }
            break;
        }
        x = next;
    }

    // 2. Step outward from the guess, alternating sides, looking for a sign
    //    change. Steps grow, reaching about ±1E8.
    for side in [1.0, -1.0] {
        let mut prev = (guess, f(guess));
        let mut step = 0.1;
        for _ in 0..120 {
            let x = prev.0 + side * step;
            let fx = f(x);
            if let (Some(a), Some(b)) = (prev.1, fx)
                && a.signum() != b.signum()
            {
                let root = bisect(&mut f, prev.0, x)?;
                // A sign change can also be a pole, as in tan; skip those.
                if is_root(&mut f, root) {
                    return Ok(root);
                }
            }
            prev = (x, fx);
            step *= 1.2;
        }
    }
    Err(EvalError::NoSolution)
}

fn bisect(
    f: &mut impl FnMut(f64) -> Option<f64>,
    mut lo: f64,
    mut hi: f64,
) -> Result<f64, EvalError> {
    let mut f_lo = f(lo).ok_or(EvalError::NoSolution)?;
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if mid == lo || mid == hi {
            break;
        }
        let f_mid = f(mid).ok_or(EvalError::NoSolution)?;
        if f_mid == 0.0 {
            return Ok(mid);
        }
        if f_mid.signum() == f_lo.signum() {
            lo = mid;
            f_lo = f_mid;
        } else {
            hi = mid;
        }
    }
    Ok(0.5 * (lo + hi))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_roots(coeffs: &[f64], expected: &[f64]) {
        let roots = polynomial_roots(coeffs).unwrap();
        assert_eq!(roots.len(), expected.len(), "{coeffs:?} gave {roots:?}");
        for (r, e) in roots.iter().zip(expected) {
            assert!((r - e).abs() < 1e-9, "{coeffs:?} gave {roots:?}");
        }
    }

    #[test]
    fn polynomials() {
        assert_roots(&[2.0, -4.0], &[2.0]); // 2x − 4
        assert_roots(&[1.0, -3.0, 2.0], &[1.0, 2.0]); // (x−1)(x−2)
        assert_roots(&[1.0, -2.0, 1.0], &[1.0]); // (x−1)²
        assert_roots(&[0.0, 1.0, -3.0, 2.0], &[1.0, 2.0]); // leading 0 ignored
        assert_roots(&[1.0, -6.0, 11.0, -6.0], &[1.0, 2.0, 3.0]); // (x−1)(x−2)(x−3)
        assert_roots(&[1.0, 0.0, 0.0, -8.0], &[2.0]); // x³ − 8
        assert_roots(&[1.0, -3.0, 3.0, -1.0], &[1.0]); // (x−1)³
        assert_roots(&[1.0, 0.0, -1.0, 0.0], &[-1.0, 0.0, 1.0]); // x³ − x
        assert_eq!(
            polynomial_roots(&[1.0, 0.0, 1.0]),
            Err(EvalError::NoRealRoots)
        );
        assert_eq!(polynomial_roots(&[0.0, 5.0]), Err(EvalError::NoSolution));
        assert_eq!(polynomial_roots(&[0.0, 0.0]), Err(EvalError::Domain));
    }

    #[test]
    fn numeric_roots() {
        let root = find_root(|x| Some(x * x - 2.0), 1.0).unwrap();
        assert!((root - 2f64.sqrt()).abs() < 1e-9);

        // Undefined left of 0: the search must step around it.
        let root = find_root(|x| (x > 0.0).then(|| x.ln() - 1.0), -5.0).unwrap();
        assert!((root - std::f64::consts::E).abs() < 1e-9);

        // Newton's method stalls at the flat point x = 0; bisection takes over.
        let root = find_root(|x| Some(x * x * x - 8.0), 0.0).unwrap();
        assert!((root - 2.0).abs() < 1e-9);

        assert_eq!(
            find_root(|x| Some(x * x + 1.0), 0.0),
            Err(EvalError::NoSolution)
        );
    }
}
