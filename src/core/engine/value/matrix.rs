//! A small dense matrix of `f64`, stored row by row.

use crate::core::engine::evaluator::EvalError;

/// Largest number of rows or columns. Keeps `iden(1E9)` from trying to
/// allocate the whole machine.
pub const MAX_DIM: usize = 64;

#[derive(Debug, Clone, PartialEq)]
pub struct Matrix {
    rows: usize,
    cols: usize,
    data: Vec<f64>,
}

impl Matrix {
    pub fn new(rows: usize, cols: usize, data: Vec<f64>) -> Result<Self, EvalError> {
        if rows == 0 || cols == 0 || rows > MAX_DIM || cols > MAX_DIM || data.len() != rows * cols {
            return Err(EvalError::DimensionMismatch);
        }
        Ok(Self { rows, cols, data })
    }

    pub fn identity(n: usize) -> Result<Self, EvalError> {
        let mut data = vec![0.0; n * n];
        for i in 0..n {
            data[i * n + i] = 1.0;
        }
        Self::new(n, n, data)
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn get(&self, row: usize, col: usize) -> f64 {
        self.data[row * self.cols + col]
    }

    /// All elements, row by row.
    pub fn values(&self) -> &[f64] {
        &self.data
    }

    pub fn map(&self, f: impl Fn(f64) -> f64) -> Self {
        Self {
            rows: self.rows,
            cols: self.cols,
            data: self.data.iter().map(|&x| f(x)).collect(),
        }
    }

    pub fn scale(&self, k: f64) -> Self {
        self.map(|x| x * k)
    }

    pub fn transpose(&self) -> Self {
        let mut data = Vec::with_capacity(self.data.len());
        for c in 0..self.cols {
            for r in 0..self.rows {
                data.push(self.get(r, c));
            }
        }
        Self {
            rows: self.cols,
            cols: self.rows,
            data,
        }
    }

    pub fn add(&self, other: &Self) -> Result<Self, EvalError> {
        self.zip(other, |a, b| a + b)
    }

    pub fn sub(&self, other: &Self) -> Result<Self, EvalError> {
        self.zip(other, |a, b| a - b)
    }

    fn zip(&self, other: &Self, f: impl Fn(f64, f64) -> f64) -> Result<Self, EvalError> {
        if self.rows != other.rows || self.cols != other.cols {
            return Err(EvalError::DimensionMismatch);
        }
        let data = self.data.iter().zip(&other.data).map(|(&a, &b)| f(a, b));
        Self::new(self.rows, self.cols, data.collect())
    }

    pub fn mul(&self, other: &Self) -> Result<Self, EvalError> {
        if self.cols != other.rows {
            return Err(EvalError::DimensionMismatch);
        }
        let mut data = vec![0.0; self.rows * other.cols];
        for r in 0..self.rows {
            for c in 0..other.cols {
                data[r * other.cols + c] = (0..self.cols)
                    .map(|k| self.get(r, k) * other.get(k, c))
                    .sum();
            }
        }
        Self::new(self.rows, other.cols, data)
    }

    pub fn determinant(&self) -> Result<f64, EvalError> {
        self.require_square()?;
        let mut a = self.data.clone();
        match gauss_jordan(&mut a, self.rows, &mut [], 0) {
            Ok(det) => Ok(det),
            Err(EvalError::SingularMatrix) => Ok(0.0),
            Err(e) => Err(e),
        }
    }

    pub fn inverse(&self) -> Result<Self, EvalError> {
        self.require_square()?;
        let n = self.rows;
        let mut a = self.data.clone();
        let mut b = Self::identity(n)?.data;
        gauss_jordan(&mut a, n, &mut b, n)?;
        Self::new(n, n, b)
    }

    /// Solves `self · x = rhs` for `x`. `rhs` may be a row or a column; the
    /// answer is always a column.
    pub fn solve(&self, rhs: &Self) -> Result<Self, EvalError> {
        self.require_square()?;
        let n = self.rows;
        if rhs.data.len() != n || (rhs.rows != 1 && rhs.cols != 1) {
            return Err(EvalError::DimensionMismatch);
        }
        let mut a = self.data.clone();
        let mut b = rhs.data.clone();
        gauss_jordan(&mut a, n, &mut b, 1)?;
        Self::new(n, 1, b)
    }

    /// Whole-number powers; negative ones use the inverse.
    pub fn pow(&self, exponent: i64) -> Result<Self, EvalError> {
        self.require_square()?;
        let mut base = if exponent < 0 {
            self.inverse()?
        } else {
            self.clone()
        };
        let mut result = Self::identity(self.rows)?;
        let mut n = exponent.unsigned_abs();
        while n > 0 {
            if n & 1 == 1 {
                result = result.mul(&base)?;
            }
            base = base.mul(&base)?;
            n >>= 1;
        }
        Ok(result)
    }

    fn require_square(&self) -> Result<(), EvalError> {
        if self.rows == self.cols {
            Ok(())
        } else {
            Err(EvalError::DimensionMismatch)
        }
    }
}

/// Gauss–Jordan elimination with partial pivoting on `[A | B]`, where `A` is
/// `n×n` and `B` is `n×m` (both row by row). Turns `B` into `A⁻¹·B` and
/// returns `det(A)`.
fn gauss_jordan(a: &mut [f64], n: usize, b: &mut [f64], m: usize) -> Result<f64, EvalError> {
    let largest = a.iter().fold(0.0_f64, |acc, x| acc.max(x.abs()));
    let tolerance = largest * 1e-12;
    let mut det = 1.0;

    for col in 0..n {
        // Pivot on the largest value in this column, for numerical stability.
        let pivot = (col..n)
            .max_by(|&i, &j| a[i * n + col].abs().total_cmp(&a[j * n + col].abs()))
            .unwrap_or(col);
        if largest == 0.0 || a[pivot * n + col].abs() <= tolerance {
            return Err(EvalError::SingularMatrix);
        }
        if pivot != col {
            for k in 0..n {
                a.swap(pivot * n + k, col * n + k);
            }
            for k in 0..m {
                b.swap(pivot * m + k, col * m + k);
            }
            det = -det;
        }

        let p = a[col * n + col];
        det *= p;
        for k in 0..n {
            a[col * n + k] /= p;
        }
        for k in 0..m {
            b[col * m + k] /= p;
        }

        for row in 0..n {
            let factor = a[row * n + col];
            if row == col || factor == 0.0 {
                continue;
            }
            for k in 0..n {
                a[row * n + k] -= factor * a[col * n + k];
            }
            for k in 0..m {
                b[row * m + k] -= factor * b[col * m + k];
            }
        }
    }
    Ok(det)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(rows: usize, cols: usize, data: &[f64]) -> Matrix {
        Matrix::new(rows, cols, data.to_vec()).unwrap()
    }

    fn close(a: &Matrix, b: &Matrix) -> bool {
        a.rows == b.rows
            && a.cols == b.cols
            && a.data
                .iter()
                .zip(&b.data)
                .all(|(x, y)| (x - y).abs() < 1e-9)
    }

    #[test]
    fn arithmetic() {
        let a = m(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        let b = m(2, 2, &[5.0, 6.0, 7.0, 8.0]);
        assert_eq!(a.add(&b).unwrap(), m(2, 2, &[6.0, 8.0, 10.0, 12.0]));
        assert_eq!(a.mul(&b).unwrap(), m(2, 2, &[19.0, 22.0, 43.0, 50.0]));
        assert_eq!(a.transpose(), m(2, 2, &[1.0, 3.0, 2.0, 4.0]));
        assert_eq!(a.pow(2).unwrap(), a.mul(&a).unwrap());
        assert_eq!(a.pow(0).unwrap(), Matrix::identity(2).unwrap());
        assert_eq!(
            a.add(&m(1, 2, &[1.0, 1.0])),
            Err(EvalError::DimensionMismatch)
        );
    }

    #[test]
    fn determinant_and_inverse() {
        let a = m(2, 2, &[4.0, 7.0, 2.0, 6.0]);
        assert!((a.determinant().unwrap() - 10.0).abs() < 1e-9);
        let inv = a.inverse().unwrap();
        assert!(close(&a.mul(&inv).unwrap(), &Matrix::identity(2).unwrap()));
        assert!(close(&a.pow(-1).unwrap(), &inv));

        let singular = m(2, 2, &[1.0, 2.0, 2.0, 4.0]);
        assert_eq!(singular.determinant(), Ok(0.0));
        assert_eq!(singular.inverse(), Err(EvalError::SingularMatrix));

        // Needs a row swap: the first pivot is 0.
        let swap = m(2, 2, &[0.0, 1.0, 1.0, 0.0]);
        assert!((swap.determinant().unwrap() + 1.0).abs() < 1e-12);
    }

    #[test]
    fn solve_linear_system() {
        // x + y = 3, x − y = 1  →  x = 2, y = 1
        let a = m(2, 2, &[1.0, 1.0, 1.0, -1.0]);
        let b = m(2, 1, &[3.0, 1.0]);
        assert!(close(&a.solve(&b).unwrap(), &m(2, 1, &[2.0, 1.0])));
        assert!(close(
            &a.solve(&b.transpose()).unwrap(),
            &m(2, 1, &[2.0, 1.0])
        ));
    }

    #[test]
    fn size_limits() {
        assert_eq!(
            Matrix::identity(MAX_DIM + 1),
            Err(EvalError::DimensionMismatch)
        );
        assert_eq!(Matrix::new(0, 0, vec![]), Err(EvalError::DimensionMismatch));
    }
}
