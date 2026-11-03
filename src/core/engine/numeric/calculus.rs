//! Numerical integration and differentiation.

use crate::core::engine::evaluator::EvalError;

/// Most function evaluations one integral may use.
const MAX_EVALUATIONS: usize = 200_000;

/// ∫ f(x) dx from `a` to `b`, by adaptive Gauss–Kronrod (7/15 points).
/// The nodes never touch the end points, so `∫ 1/√x` from 0 works.
/// `f` returns `None` where it is undefined.
pub fn integrate(f: &mut impl FnMut(f64) -> Option<f64>, a: f64, b: f64) -> Result<f64, EvalError> {
    if a == b {
        return Ok(0.0);
    }
    let mut evaluations = 0;
    let (whole, error) = kronrod(f, a, b, &mut evaluations)?;
    let tolerance = 1e-11 * whole.abs().max(1.0);
    let total = refine(f, a, b, whole, error, tolerance, 50, &mut evaluations)?;
    if total.is_finite() {
        Ok(total)
    } else {
        Err(EvalError::InvalidResult)
    }
}

#[allow(clippy::too_many_arguments)]
fn refine(
    f: &mut impl FnMut(f64) -> Option<f64>,
    a: f64,
    b: f64,
    estimate: f64,
    error: f64,
    tolerance: f64,
    depth: u32,
    evaluations: &mut usize,
) -> Result<f64, EvalError> {
    if error <= tolerance || depth == 0 {
        return Ok(estimate);
    }
    let mid = 0.5 * (a + b);
    let (left, left_err) = kronrod(f, a, mid, evaluations)?;
    let (right, right_err) = kronrod(f, mid, b, evaluations)?;
    Ok(refine(
        f,
        a,
        mid,
        left,
        left_err,
        tolerance / 2.0,
        depth - 1,
        evaluations,
    )? + refine(
        f,
        mid,
        b,
        right,
        right_err,
        tolerance / 2.0,
        depth - 1,
        evaluations,
    )?)
}

/// One Gauss–Kronrod step: the 15-point estimate and its error.
fn kronrod(
    f: &mut impl FnMut(f64) -> Option<f64>,
    a: f64,
    b: f64,
    evaluations: &mut usize,
) -> Result<(f64, f64), EvalError> {
    const XGK: [f64; 8] = [
        0.991_455_371_120_812_6,
        0.949_107_912_342_758_5,
        0.864_864_423_359_769_1,
        0.741_531_185_599_394_4,
        0.586_087_235_467_691_1,
        0.405_845_151_377_397_2,
        0.207_784_955_007_898_5,
        0.0,
    ];
    const WGK: [f64; 8] = [
        0.022_935_322_010_529_22,
        0.063_092_092_629_978_55,
        0.104_790_010_322_250_2,
        0.140_653_259_715_525_9,
        0.169_004_726_639_267_9,
        0.190_350_578_064_785_4,
        0.204_432_940_075_298_9,
        0.209_482_141_084_727_8,
    ];
    const WG: [f64; 4] = [
        0.129_484_966_168_869_7,
        0.279_705_391_489_276_7,
        0.381_830_050_505_118_9,
        0.417_959_183_673_469_4,
    ];

    *evaluations += 15;
    if *evaluations > MAX_EVALUATIONS {
        return Err(EvalError::LimitExceeded);
    }
    let center = 0.5 * (a + b);
    let half = 0.5 * (b - a);
    let mut at = |x: f64| f(x).filter(|y| y.is_finite()).ok_or(EvalError::Domain);

    let fc = at(center)?;
    let mut kronrod = fc * WGK[7];
    let mut gauss = fc * WG[3];
    for j in 0..7 {
        let dx = half * XGK[j];
        let pair = at(center - dx)? + at(center + dx)?;
        kronrod += WGK[j] * pair;
        if j % 2 == 1 {
            gauss += WG[j / 2] * pair;
        }
    }
    Ok((kronrod * half, ((kronrod - gauss) * half).abs()))
}

/// f′(x), by Ridders' method: central differences with shrinking steps,
/// combined by Richardson extrapolation.
pub fn derivative(f: &mut impl FnMut(f64) -> Option<f64>, x: f64) -> Result<f64, EvalError> {
    const SIZE: usize = 10;
    const SHRINK: f64 = 1.4;
    let mut at = |x: f64| f(x).filter(|y| y.is_finite()).ok_or(EvalError::Domain);

    let mut h = 0.1 * x.abs().max(1.0);
    let mut table = [[0.0_f64; SIZE]; SIZE];
    table[0][0] = (at(x + h)? - at(x - h)?) / (2.0 * h);
    let mut best = table[0][0];
    let mut best_error = f64::INFINITY;

    for i in 1..SIZE {
        h /= SHRINK;
        table[0][i] = (at(x + h)? - at(x - h)?) / (2.0 * h);
        let mut factor = SHRINK * SHRINK;
        for j in 1..=i {
            table[j][i] = (table[j - 1][i] * factor - table[j - 1][i - 1]) / (factor - 1.0);
            factor *= SHRINK * SHRINK;
            let error = (table[j][i] - table[j - 1][i])
                .abs()
                .max((table[j][i] - table[j - 1][i - 1]).abs());
            if error <= best_error {
                best_error = error;
                best = table[j][i];
            }
        }
        // Stop once higher orders start getting worse.
        if (table[i][i] - table[i - 1][i - 1]).abs() >= 2.0 * best_error {
            break;
        }
    }
    if best.is_finite() {
        Ok(best)
    } else {
        Err(EvalError::InvalidResult)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * b.abs().max(1.0)
    }

    #[test]
    fn integrals() {
        assert!(close(
            integrate(&mut |x| Some(x * x), 0.0, 3.0).unwrap(),
            9.0
        ));
        assert!(close(
            integrate(&mut |x| Some(x.sin()), 0.0, std::f64::consts::PI).unwrap(),
            2.0
        ));
        // Singular at the end point, but integrable: ∫₀¹ 1/√x dx = 2.
        let r = integrate(&mut |x| Some(1.0 / x.sqrt()), 0.0, 1.0).unwrap();
        assert!((r - 2.0).abs() < 1e-6, "{r}");
        assert!(close(integrate(&mut |x| Some(x), 2.0, 0.0).unwrap(), -2.0));
        assert_eq!(integrate(&mut |x| Some(x), 1.0, 1.0), Ok(0.0));
        assert_eq!(
            integrate(&mut |x| (x > 0.5).then_some(x), 0.0, 1.0),
            Err(EvalError::Domain)
        );
    }

    #[test]
    fn derivatives() {
        assert!(close(
            derivative(&mut |x| Some(x * x * x), 2.0).unwrap(),
            12.0
        ));
        assert!(close(derivative(&mut |x| Some(x.sin()), 0.0).unwrap(), 1.0));
        assert!(close(
            derivative(&mut |x| Some(x.exp()), 1.0).unwrap(),
            std::f64::consts::E
        ));
        assert!(close(derivative(&mut |x| Some(x.ln()), 10.0).unwrap(), 0.1));
    }
}
