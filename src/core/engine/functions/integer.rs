//! Whole-number functions: division, combinatorics and bitwise logic.
//!
//! The bitwise functions work on two's-complement words of
//! `settings.word_bits` bits, so `not(0)` is −1 and the result always fits
//! the word.

use super::{FunctionDef, VARIADIC, eager, finite, real, real2, whole};
use crate::core::engine::{context::Context, evaluator::EvalError, value::Value};

pub(super) static DEFS: &[FunctionDef] = &[
    eager(
        "mod",
        &[],
        (2, 2),
        "mod(a, b): remainder of a ÷ b, with the sign of b",
        |a, _| {
            real2(&a[0], &a[1], |x, y| {
                if y == 0.0 {
                    return Err(EvalError::DivisionByZero);
                }
                Ok(x - y * (x / y).floor())
            })
        },
    ),
    eager(
        "gcd",
        &[],
        (1, VARIADIC),
        "gcd(a, b, …): greatest common divisor",
        |a, _| {
            let mut result = 0_u64;
            for n in integers(a)? {
                result = gcd(result, n.unsigned_abs());
            }
            Ok(Value::Number(result as f64))
        },
    ),
    eager(
        "lcm",
        &[],
        (1, VARIADIC),
        "lcm(a, b, …): least common multiple",
        |a, _| {
            let mut result = 1_u64;
            for n in integers(a)? {
                let n = n.unsigned_abs();
                if n == 0 {
                    return Ok(Value::Number(0.0));
                }
                result = (result / gcd(result, n))
                    .checked_mul(n)
                    .ok_or(EvalError::InvalidResult)?;
            }
            Ok(Value::Number(result as f64))
        },
    ),
    eager("nPr", &[], (2, 2), "nPr(n, r): permutations", |a, _| {
        real2(&a[0], &a[1], |n, r| {
            let (n, r) = n_r(n, r)?;
            finite(((n - r + 1)..=n).map(|k| k as f64).product())
        })
    }),
    eager("nCr", &[], (2, 2), "nCr(n, r): combinations", |a, _| {
        real2(&a[0], &a[1], |n, r| {
            let (n, r) = n_r(n, r)?;
            let r = r.min(n - r);
            let mut result = 1.0_f64;
            for k in 1..=r {
                result = result * (n - r + k) as f64 / k as f64;
            }
            finite(result.round())
        })
    }),
    eager(
        "isprime",
        &[],
        (1, 1),
        "isprime(n): 1 if n is prime, else 0",
        |a, _| {
            real(&a[0], |x| {
                let n = whole(x)?;
                Ok(if is_prime(n) { 1.0 } else { 0.0 })
            })
        },
    ),
    // bitwise
    eager("and", &[], (2, 2), "and(a, b): bitwise AND", |a, c| {
        bits2(a, c, |x, y| x & y)
    }),
    eager("or", &[], (2, 2), "or(a, b): bitwise OR", |a, c| {
        bits2(a, c, |x, y| x | y)
    }),
    eager(
        "xor",
        &[],
        (2, 2),
        "xor(a, b): bitwise exclusive OR",
        |a, c| bits2(a, c, |x, y| x ^ y),
    ),
    eager(
        "xnor",
        &[],
        (2, 2),
        "xnor(a, b): bitwise exclusive NOR",
        |a, c| bits2(a, c, |x, y| !(x ^ y)),
    ),
    eager("not", &[], (1, 1), "not(a): bitwise NOT", |a, c| {
        let bits = c.settings.word_bits;
        real(&a[0], |x| Ok(from_word(!to_word(x, bits)?, bits)))
    }),
    eager(
        "neg",
        &[],
        (1, 1),
        "neg(a): two's-complement negation",
        |a, c| {
            let bits = c.settings.word_bits;
            real(&a[0], |x| {
                Ok(from_word(to_word(x, bits)?.wrapping_neg(), bits))
            })
        },
    ),
    eager(
        "shl",
        &[],
        (2, 2),
        "shl(a, n): shift left by n bits",
        |a, c| bits2(a, c, |x, n| x.checked_shl(n as u32).unwrap_or(0)),
    ),
    eager(
        "shr",
        &[],
        (2, 2),
        "shr(a, n): logical shift right by n bits",
        |a, c| {
            let bits = c.settings.word_bits;
            bits2(a, c, move |x, n| {
                (x & mask(bits)).checked_shr(n as u32).unwrap_or(0)
            })
        },
    ),
];

fn integers(args: &[Value]) -> Result<Vec<i64>, EvalError> {
    let mut out = Vec::new();
    for v in args {
        for x in v.numbers()? {
            out.push(whole(x)?);
        }
    }
    Ok(out)
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// Checks `0 ≤ r ≤ n` for nPr and nCr.
fn n_r(n: f64, r: f64) -> Result<(u64, u64), EvalError> {
    let (n, r) = (whole(n)?, whole(r)?);
    if n < 0 || r < 0 || r > n {
        return Err(EvalError::Domain);
    }
    Ok((n as u64, r as u64))
}

fn is_prime(n: i64) -> bool {
    if n < 2 {
        return false;
    }
    if n % 2 == 0 {
        return n == 2;
    }
    let mut d = 3;
    while d * d <= n {
        if n % d == 0 {
            return false;
        }
        d += 2;
    }
    true
}

fn mask(bits: u8) -> u64 {
    if bits >= 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    }
}

/// A whole number as a word. Accepts anything that fits as signed or
/// unsigned, e.g. −1 and 255 for 8 bits.
fn to_word(x: f64, bits: u8) -> Result<u64, EvalError> {
    let n = whole(x)? as i128;
    let lowest = -(1i128 << (bits - 1));
    let highest = (1i128 << bits) - 1;
    if !(lowest..=highest).contains(&n) {
        return Err(EvalError::Domain);
    }
    Ok((n as u128 as u64) & mask(bits))
}

/// A word read back as a signed number.
fn from_word(w: u64, bits: u8) -> f64 {
    let w = w & mask(bits);
    let sign_bit = 1u64 << (bits - 1);
    if w & sign_bit == 0 {
        w as f64
    } else {
        (w as i128 - (1i128 << bits)) as f64
    }
}

fn bits2(args: &[Value], ctx: &Context, f: impl Fn(u64, u64) -> u64) -> Result<Value, EvalError> {
    let bits = ctx.settings.word_bits;
    real2(&args[0], &args[1], |x, y| {
        Ok(from_word(f(to_word(x, bits)?, to_word(y, bits)?), bits))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words() {
        assert_eq!(to_word(-1.0, 8), Ok(0xFF));
        assert_eq!(to_word(255.0, 8), Ok(0xFF));
        assert_eq!(to_word(256.0, 8), Err(EvalError::Domain));
        assert_eq!(from_word(0xFF, 8), -1.0);
        assert_eq!(from_word(0x7F, 8), 127.0);
        assert_eq!(from_word(u64::MAX, 64), -1.0);
        assert_eq!(to_word(-1.0, 64), Ok(u64::MAX));
    }

    #[test]
    fn primes() {
        let primes: Vec<i64> = (0..30).filter(|&n| is_prime(n)).collect();
        assert_eq!(primes, [2, 3, 5, 7, 11, 13, 17, 19, 23, 29]);
    }
}
