use crate::core::engine::evaluator::EvalError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub const I: Self = Self { re: 0.0, im: 1.0 };

    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    pub fn from_polar(r: f64, theta: f64) -> Self {
        Self::new(r * theta.cos(), r * theta.sin())
    }

    pub fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }

    /// The angle in radians, in (−π, π].
    pub fn arg(self) -> f64 {
        self.im.atan2(self.re)
    }

    pub fn conj(self) -> Self {
        Self::new(self.re, -self.im)
    }

    /// Division, failing on a zero divisor instead of giving infinity.
    pub fn checked_div(self, o: Self) -> Result<Self, EvalError> {
        if o.re == 0.0 && o.im == 0.0 {
            return Err(EvalError::DivisionByZero);
        }
        // Smith's method: avoids overflow when |o| is very large or small.
        Ok(if o.re.abs() >= o.im.abs() {
            let r = o.im / o.re;
            let d = o.re + o.im * r;
            Self::new((self.re + self.im * r) / d, (self.im - self.re * r) / d)
        } else {
            let r = o.re / o.im;
            let d = o.re * r + o.im;
            Self::new((self.re * r + self.im) / d, (self.im * r - self.re) / d)
        })
    }

    pub fn exp(self) -> Self {
        Self::from_polar(self.re.exp(), self.im)
    }

    /// The principal natural logarithm.
    pub fn ln(self) -> Result<Self, EvalError> {
        if self.re == 0.0 && self.im == 0.0 {
            return Err(EvalError::Domain);
        }
        Ok(Self::new(self.abs().ln(), self.arg()))
    }

    /// The principal square root.
    pub fn sqrt(self) -> Self {
        let r = self.abs();
        let re = ((r + self.re) / 2.0).sqrt();
        let im = ((r - self.re) / 2.0).sqrt().copysign(self.im);
        Self::new(re, im)
    }

    /// The principal value of `self ^ exponent`. Whole-number exponents
    /// use repeated multiplication, so `(2i)^2` is exactly −4.
    pub fn pow(self, exponent: Self) -> Result<Self, EvalError> {
        if exponent.im == 0.0 && exponent.re.fract() == 0.0 && exponent.re.abs() <= 1024.0 {
            let n = exponent.re as i64;
            let mut result = Self::new(1.0, 0.0);
            let mut base = self;
            let mut k = n.unsigned_abs();
            while k > 0 {
                if k & 1 == 1 {
                    result = result * base;
                }
                base = base * base;
                k >>= 1;
            }
            return if n < 0 {
                Self::new(1.0, 0.0).checked_div(result)
            } else {
                Ok(result)
            };
        }
        if self.re == 0.0 && self.im == 0.0 {
            return if exponent.re > 0.0 {
                Ok(self)
            } else {
                Err(EvalError::Domain)
            };
        }
        Ok((exponent * self.ln()?).exp())
    }
}

impl std::ops::Neg for Complex {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.re, -self.im)
    }
}

impl std::ops::Add for Complex {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self::new(self.re + o.re, self.im + o.im)
    }
}

impl std::ops::Sub for Complex {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Self::new(self.re - o.re, self.im - o.im)
    }
}

impl std::ops::Mul for Complex {
    type Output = Self;
    fn mul(self, o: Self) -> Self {
        Self::new(
            self.re * o.re - self.im * o.im,
            self.re * o.im + self.im * o.re,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Complex, re: f64, im: f64) -> bool {
        (a.re - re).abs() < 1e-12 && (a.im - im).abs() < 1e-12
    }

    #[test]
    fn arithmetic() {
        let a = Complex::new(3.0, 4.0);
        let b = Complex::new(1.0, -2.0);
        assert!(close(a + b, 4.0, 2.0));
        assert!(close(a - b, 2.0, 6.0));
        assert!(close(a * b, 11.0, -2.0));
        assert!(close(-a, -3.0, -4.0));
        assert!(close(a.checked_div(b).unwrap(), -1.0, 2.0));
        assert_eq!(a.abs(), 5.0);
        assert_eq!(
            a.checked_div(Complex::new(0.0, 0.0)),
            Err(EvalError::DivisionByZero)
        );
    }

    #[test]
    fn functions() {
        assert!(close(Complex::new(-4.0, 0.0).sqrt(), 0.0, 2.0));
        assert!(close(
            Complex::I.pow(Complex::new(2.0, 0.0)).unwrap(),
            -1.0,
            0.0
        ));
        let euler = Complex::new(0.0, std::f64::consts::PI).exp();
        assert!(close(euler, -1.0, 0.0));
        let ln = Complex::new(-1.0, 0.0).ln().unwrap();
        assert!(close(ln, 0.0, std::f64::consts::PI));
        // (−8)^(1/3): the principal root is 1 + √3·i.
        let root = Complex::new(-8.0, 0.0)
            .pow(Complex::new(1.0 / 3.0, 0.0))
            .unwrap();
        assert!(close(root, 1.0, 3f64.sqrt()));
    }
}
