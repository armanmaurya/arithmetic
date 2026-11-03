//! Special functions: gamma, the error function and the normal distribution.

use std::f64::consts::PI;

/// ln Γ(x) for x > 0 (Lanczos approximation, g = 7, about 15 digits).
pub fn ln_gamma(x: f64) -> f64 {
    const G: f64 = 7.0;
    const COEFFS: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        // Reflection: Γ(x)Γ(1−x) = π / sin(πx)
        return (PI / (PI * x).sin()).abs().ln() - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let mut sum = COEFFS[0];
    for (i, &c) in COEFFS.iter().enumerate().skip(1) {
        sum += c / (x + i as f64);
    }
    let t = x + G + 0.5;
    0.5 * (2.0 * PI).ln() + (x + 0.5) * t.ln() - t + sum.ln()
}

/// Γ(x). `None` at 0 and the negative integers, where it is undefined.
pub fn gamma(x: f64) -> Option<f64> {
    if x <= 0.0 && x.fract() == 0.0 {
        return None;
    }
    if x.fract() == 0.0 && x <= 171.0 {
        // Exact for whole numbers: Γ(n) = (n−1)!
        return Some((1..x as u64).map(|k| k as f64).product());
    }
    if x < 0.5 {
        return Some(PI / ((PI * x).sin() * gamma(1.0 - x)?));
    }
    Some(ln_gamma(x).exp())
}

/// The error function, accurate to about 1E-13.
pub fn erf(x: f64) -> f64 {
    if x.abs() < 2.5 {
        // Taylor series: 2/√π · Σ (−1)ⁿ x^(2n+1) / (n! (2n+1))
        let mut term = x;
        let mut sum = x;
        for n in 1..200 {
            term *= -x * x / n as f64;
            let add = term / (2 * n + 1) as f64;
            sum += add;
            if add.abs() < 1e-17 * sum.abs() {
                break;
            }
        }
        2.0 / PI.sqrt() * sum
    } else {
        x.signum() * (1.0 - erfc_large(x.abs()))
    }
}

/// 1 − erf(x), accurate even where erf(x) is close to 1.
pub fn erfc(x: f64) -> f64 {
    if x >= 2.5 {
        erfc_large(x)
    } else if x <= -2.5 {
        2.0 - erfc_large(-x)
    } else {
        1.0 - erf(x)
    }
}

/// erfc for x ≥ 2.5, from its continued fraction.
fn erfc_large(x: f64) -> f64 {
    let mut f = x;
    for n in (1..=120).rev() {
        f = x + (n as f64 / 2.0) / f;
    }
    (-x * x).exp() / (PI.sqrt() * f)
}

/// P(Z ≤ z) for the standard normal distribution.
pub fn normal_cdf(z: f64) -> f64 {
    0.5 * erfc(-z / std::f64::consts::SQRT_2)
}

pub fn normal_pdf(z: f64) -> f64 {
    (-0.5 * z * z).exp() / (2.0 * PI).sqrt()
}

/// The z with P(Z ≤ z) = p, for 0 < p < 1 (Acklam's approximation, then
/// two Newton steps).
pub fn inverse_normal(p: f64) -> Option<f64> {
    if !(p > 0.0 && p < 1.0) {
        return None;
    }
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];
    let tail = |q: f64| {
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    };
    let mut z = if p < 0.024_25 {
        tail((-2.0 * p.ln()).sqrt())
    } else if p > 1.0 - 0.024_25 {
        -tail((-2.0 * (1.0 - p).ln()).sqrt())
    } else {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    };
    for _ in 0..2 {
        let pdf = normal_pdf(z);
        if pdf > 0.0 {
            z -= (normal_cdf(z) - p) / pdf;
        }
    }
    Some(z)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol * b.abs().max(1.0)
    }

    #[test]
    fn gamma_values() {
        assert_eq!(gamma(5.0), Some(24.0));
        assert!(close(gamma(0.5).unwrap(), PI.sqrt(), 1e-13));
        assert!(close(gamma(-0.5).unwrap(), -2.0 * PI.sqrt(), 1e-13));
        assert_eq!(gamma(0.0), None);
        assert_eq!(gamma(-3.0), None);
        assert!(close(ln_gamma(100.0), 359.134_205_369_575_4, 1e-13));
    }

    #[test]
    fn erf_and_normal() {
        assert!(close(erf(0.5), 0.520_499_877_813_046_5, 1e-13));
        assert!(close(erf(3.0), 0.999_977_909_503_001_4, 1e-13));
        assert!(close(erfc(5.0), 1.537_459_794_428_035e-12, 1e-10));
        assert!(close(normal_cdf(1.96), 0.975_002_104_851_780, 1e-12));
        assert!(close(normal_cdf(-1.0), 0.158_655_253_931_457, 1e-12));
        assert!(close(
            inverse_normal(0.975).unwrap(),
            1.959_963_984_540_054,
            1e-12
        ));
        assert!(close(
            inverse_normal(0.001).unwrap(),
            -3.090_232_306_167_813_5,
            1e-12
        ));
        assert_eq!(inverse_normal(1.0), None);
    }
}
