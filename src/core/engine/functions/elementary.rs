//! Trigonometry, powers, logarithms, rounding and complex parts.

use super::{FunctionDef, VARIADIC, arg_or, eager, finite, real, real2, whole};
use crate::core::engine::{
    context::Context,
    evaluator::EvalError,
    numeric::special,
    settings::AngleMode,
    value::{Complex, Value, broadcast},
};

pub(super) static DEFS: &[FunctionDef] = &[
    // trigonometry, in the current angle unit
    eager("sin", &[], (1, 1), "sin(x): sine", |a, c| {
        let m = c.settings.angle;
        real(&a[0], |x| Ok(sin_cos(x, m)?.0))
    }),
    eager("cos", &[], (1, 1), "cos(x): cosine", |a, c| {
        let m = c.settings.angle;
        real(&a[0], |x| Ok(sin_cos(x, m)?.1))
    }),
    eager("tan", &[], (1, 1), "tan(x): tangent", |a, c| {
        let m = c.settings.angle;
        real(&a[0], |x| tan(x, m))
    }),
    eager(
        "asin",
        &["sin⁻¹", "arcsin"],
        (1, 1),
        "asin(x): inverse sine",
        |a, c| {
            let m = c.settings.angle;
            real(&a[0], |x| Ok(m.from_radians(unit_interval(x)?.asin())))
        },
    ),
    eager(
        "acos",
        &["cos⁻¹", "arccos"],
        (1, 1),
        "acos(x): inverse cosine",
        |a, c| {
            let m = c.settings.angle;
            real(&a[0], |x| Ok(m.from_radians(unit_interval(x)?.acos())))
        },
    ),
    eager(
        "atan",
        &["tan⁻¹", "arctan"],
        (1, 1),
        "atan(x): inverse tangent",
        |a, c| {
            let m = c.settings.angle;
            real(&a[0], |x| Ok(m.from_radians(x.atan())))
        },
    ),
    eager("sinh", &[], (1, 1), "sinh(x): hyperbolic sine", |a, _| {
        real(&a[0], |x| finite(x.sinh()))
    }),
    eager("cosh", &[], (1, 1), "cosh(x): hyperbolic cosine", |a, _| {
        real(&a[0], |x| finite(x.cosh()))
    }),
    eager(
        "tanh",
        &[],
        (1, 1),
        "tanh(x): hyperbolic tangent",
        |a, _| real(&a[0], |x| Ok(x.tanh())),
    ),
    eager(
        "asinh",
        &["sinh⁻¹", "arsinh"],
        (1, 1),
        "asinh(x): inverse hyperbolic sine",
        |a, _| real(&a[0], |x| Ok(x.asinh())),
    ),
    eager(
        "acosh",
        &["cosh⁻¹", "arcosh"],
        (1, 1),
        "acosh(x): inverse hyperbolic cosine, x ≥ 1",
        |a, _| {
            real(&a[0], |x| {
                if x >= 1.0 {
                    Ok(x.acosh())
                } else {
                    Err(EvalError::Domain)
                }
            })
        },
    ),
    eager(
        "atanh",
        &["tanh⁻¹", "artanh"],
        (1, 1),
        "atanh(x): inverse hyperbolic tangent, |x| < 1",
        |a, _| {
            real(&a[0], |x| {
                if x.abs() < 1.0 {
                    Ok(x.atanh())
                } else {
                    Err(EvalError::Domain)
                }
            })
        },
    ),
    // powers, roots and logarithms
    eager("sqrt", &["√"], (1, 1), "sqrt(x): square root", |a, c| {
        either(
            &a[0],
            c,
            EvalError::NegativeRoot,
            |x| (x >= 0.0).then(|| x.sqrt()),
            |z| Ok(z.sqrt()),
        )
    }),
    eager("cbrt", &["∛"], (1, 1), "cbrt(x): cube root", |a, _| {
        real(&a[0], |x| Ok(x.cbrt()))
    }),
    eager("root", &[], (2, 2), "root(x, n): n-th root", |a, c| {
        let complex = c.settings.complex;
        crate::core::engine::value::broadcast2(&a[0], &a[1], &mut |x, n| {
            nth_root(x.as_number()?, n.as_number()?, complex)
        })
    }),
    eager("exp", &[], (1, 1), "exp(x): e to the power x", |a, c| {
        either(
            &a[0],
            c,
            EvalError::InvalidResult,
            |x| Some(x.exp()),
            |z| Ok(z.exp()),
        )
    }),
    eager("ln", &[], (1, 1), "ln(x): natural logarithm", |a, c| {
        either(
            &a[0],
            c,
            EvalError::Domain,
            |x| (x > 0.0).then(|| x.ln()),
            Complex::ln,
        )
    }),
    eager(
        "log",
        &[],
        (1, 2),
        "log(x) or log(x, base): logarithm, base 10 by default",
        |a, c| {
            if let Some(base) = a.get(1) {
                return real2(&a[0], base, |x, b| {
                    if x <= 0.0 || b <= 0.0 || b == 1.0 {
                        return Err(EvalError::Domain);
                    }
                    Ok(x.ln() / b.ln())
                });
            }
            either(
                &a[0],
                c,
                EvalError::Domain,
                |x| (x > 0.0).then(|| x.log10()),
                |z| {
                    let ln = z.ln()?;
                    Ok(Complex::new(
                        ln.re / std::f64::consts::LN_10,
                        ln.im / std::f64::consts::LN_10,
                    ))
                },
            )
        },
    ),
    eager("log2", &[], (1, 1), "log2(x): base-2 logarithm", |a, _| {
        real(&a[0], |x| {
            if x > 0.0 {
                Ok(x.log2())
            } else {
                Err(EvalError::Domain)
            }
        })
    }),
    // parts and rounding
    eager(
        "abs",
        &[],
        (1, 1),
        "abs(x): absolute value, or modulus of a complex number",
        |a, _| {
            broadcast(&a[0], &mut |v| match v {
                Value::Complex(z) => Ok(Value::Number(z.abs())),
                v => Ok(Value::Number(v.as_number()?.abs())),
            })
        },
    ),
    eager("sign", &["sgn"], (1, 1), "sign(x): −1, 0 or 1", |a, _| {
        real(&a[0], |x| Ok(if x == 0.0 { 0.0 } else { x.signum() }))
    }),
    eager("floor", &[], (1, 1), "floor(x): round down", |a, _| {
        real(&a[0], |x| Ok(x.floor()))
    }),
    eager("ceil", &[], (1, 1), "ceil(x): round up", |a, _| {
        real(&a[0], |x| Ok(x.ceil()))
    }),
    eager("int", &["trunc"], (1, 1), "int(x): integer part", |a, _| {
        real(&a[0], |x| Ok(x.trunc()))
    }),
    eager("frac", &[], (1, 1), "frac(x): fractional part", |a, _| {
        real(&a[0], |x| Ok(x.fract()))
    }),
    eager(
        "round",
        &[],
        (1, 2),
        "round(x) or round(x, places): round to nearest",
        |a, _| {
            let places = whole(arg_or(a, 1, 0.0)?)?;
            if !(-15..=15).contains(&places) {
                return Err(EvalError::Domain);
            }
            let scale = 10f64.powi(places as i32);
            real(&a[0], |x| Ok((x * scale).round() / scale))
        },
    ),
    eager(
        "gamma",
        &["Γ"],
        (1, 1),
        "gamma(x): the gamma function, gamma(n) = (n−1)!",
        |a, _| {
            real(&a[0], |x| {
                special::gamma(x).ok_or(EvalError::Domain).and_then(finite)
            })
        },
    ),
    // complex parts
    eager("re", &["Re"], (1, 1), "re(z): real part", |a, _| {
        broadcast(&a[0], &mut |v| Ok(Value::Number(v.as_complex()?.re)))
    }),
    eager("im", &["Im"], (1, 1), "im(z): imaginary part", |a, _| {
        broadcast(&a[0], &mut |v| Ok(Value::Number(v.as_complex()?.im)))
    }),
    eager(
        "arg",
        &[],
        (1, 1),
        "arg(z): argument, in the current angle unit",
        |a, c| {
            let m = c.settings.angle;
            broadcast(&a[0], &mut |v| {
                let z = v.as_complex()?;
                if z.re == 0.0 && z.im == 0.0 {
                    return Err(EvalError::Domain);
                }
                Ok(Value::Number(m.from_radians(z.arg())))
            })
        },
    ),
    eager("conj", &[], (1, 1), "conj(z): complex conjugate", |a, _| {
        broadcast(&a[0], &mut |v| Ok(Value::Complex(v.as_complex()?.conj())))
    }),
    // coordinates
    eager(
        "pol",
        &["Pol"],
        (2, 2),
        "pol(x, y): rectangular to polar, gives {r, θ}",
        |a, c| {
            let (x, y) = (a[0].as_number()?, a[1].as_number()?);
            if x == 0.0 && y == 0.0 {
                return Err(EvalError::Domain);
            }
            Value::list_of_numbers([x.hypot(y), c.settings.angle.from_radians(y.atan2(x))])
        },
    ),
    eager(
        "rec",
        &["Rec"],
        (2, 2),
        "rec(r, θ): polar to rectangular, gives {x, y}",
        |a, c| {
            let (r, theta) = (a[0].as_number()?, a[1].as_number()?);
            let (sin, cos) = sin_cos(theta, c.settings.angle)?;
            Value::list_of_numbers([r * cos, r * sin])
        },
    ),
    // random numbers
    eager(
        "rand",
        &["ran#", "Ran#"],
        (0, 0),
        "rand: a random number from 0 to 1",
        |_, c| Ok(Value::Number(c.next_random())),
    ),
    eager(
        "randint",
        &["ranint", "RanInt"],
        (2, 2),
        "randint(a, b): a random whole number from a to b",
        |a, c| {
            let (lo, hi) = (whole(a[0].as_number()?)?, whole(a[1].as_number()?)?);
            if lo > hi {
                return Err(EvalError::Domain);
            }
            let span = (hi - lo + 1) as f64;
            Ok(Value::Number(lo as f64 + (c.next_random() * span).floor()))
        },
    ),
    eager(
        "hypot",
        &[],
        (1, VARIADIC),
        "hypot(a, b, …): √(a² + b² + …)",
        |a, _| {
            let mut sum = 0.0;
            for v in a {
                for x in v.numbers()? {
                    sum += x * x;
                }
            }
            Ok(Value::Number(finite(sum.sqrt())?))
        },
    ),
];

/// Beyond this many radians an angle has no meaningful digits left (the
/// gap between neighbouring numbers is more than a full turn).
const MAX_RADIANS: f64 = 1e15;

/// `(sin, cos)`, exact at whole right angles: converting to radians first
/// would make cos(90°) about 6E-17, and π itself is rounded, so sin(π)
/// would be 1.2E-16.
pub(super) fn sin_cos(x: f64, angle: AngleMode) -> Result<(f64, f64), EvalError> {
    if angle.to_radians(x).abs() >= MAX_RADIANS {
        return Err(EvalError::Domain);
    }
    if let Some(k) = right_angles(x, angle) {
        return Ok([(0.0, 1.0), (1.0, 0.0), (0.0, -1.0), (-1.0, 0.0)][k]);
    }
    let radians = match angle {
        AngleMode::Radians => x,
        // Reduce first: exact in degrees, and keeps big angles accurate.
        _ => angle.to_radians(x.rem_euclid(angle.full_turn())),
    };
    Ok((radians.sin(), radians.cos()))
}

/// tan(90°) is undefined rather than huge.
fn tan(x: f64, angle: AngleMode) -> Result<f64, EvalError> {
    let (sin, cos) = sin_cos(x, angle)?;
    match right_angles(x, angle) {
        Some(k) if k % 2 == 1 => Err(EvalError::Domain),
        Some(_) => Ok(0.0),
        None => finite(sin / cos),
    }
}

/// If `x` is a whole number of right angles, that number modulo 4. In
/// radians, being within rounding error of k·π/2 counts.
fn right_angles(x: f64, angle: AngleMode) -> Option<usize> {
    let quarter = angle.full_turn() / 4.0;
    let k = (x / quarter).round();
    let exact = match angle {
        AngleMode::Radians => (x - k * quarter).abs() <= 4.0 * f64::EPSILON * x.abs(),
        _ => k * quarter == x,
    };
    exact.then(|| k.rem_euclid(4.0) as usize)
}

/// Accepts −1 ≤ x ≤ 1, snapping values a hair outside (rounding noise) in.
fn unit_interval(x: f64) -> Result<f64, EvalError> {
    if x.abs() <= 1.0 {
        Ok(x)
    } else if x.abs() <= 1.0 + 1e-12 {
        Ok(x.signum())
    } else {
        Err(EvalError::Domain)
    }
}

/// For functions with a real and a complex version (√, ln, exp, log).
/// `real` returns `None` where the answer isn't real; complex mode then
/// uses `complex` instead, and real mode fails with `error`. Complex input
/// always uses `complex`.
fn either(
    v: &Value,
    ctx: &Context,
    error: EvalError,
    real: impl Fn(f64) -> Option<f64>,
    complex: impl Fn(Complex) -> Result<Complex, EvalError>,
) -> Result<Value, EvalError> {
    let complex_mode = ctx.settings.complex;
    broadcast(v, &mut |x| match x {
        Value::Complex(z) if z.im != 0.0 => Ok(Value::Complex(complex(*z)?)),
        x => {
            let r = x.as_number()?;
            match real(r) {
                Some(y) => Ok(Value::Number(finite(y)?)),
                None if complex_mode => Ok(Value::Complex(complex(Complex::new(r, 0.0))?)),
                None => Err(error),
            }
        }
    })
}

/// The n-th root. Odd roots of negative numbers are real: root(−8, 3) = −2.
fn nth_root(x: f64, n: f64, complex: bool) -> Result<Value, EvalError> {
    if n == 0.0 {
        return Err(EvalError::Domain);
    }
    if x >= 0.0 {
        return Ok(Value::Number(finite(x.powf(1.0 / n))?));
    }
    if let Ok(k) = whole(n)
        && k % 2 != 0
    {
        return Ok(Value::Number(finite(-(-x).powf(1.0 / n))?));
    }
    if complex {
        return Ok(Value::Complex(
            Complex::new(x, 0.0).pow(Complex::new(1.0 / n, 0.0))?,
        ));
    }
    Err(EvalError::NegativeRoot)
}
