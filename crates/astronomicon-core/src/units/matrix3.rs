use crate::units::vector3::Vector3;
use serde::{ Deserialize, Serialize };

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Matrix3(pub [[f64; 3]; 3]);

impl Matrix3 {
    pub fn new(
        m00: f64,
        m01: f64,
        m02: f64,
        m10: f64,
        m11: f64,
        m12: f64,
        m20: f64,
        m21: f64,
        m22: f64
    ) -> Self {
        Self([
            [m00, m01, m02],
            [m10, m11, m12],
            [m20, m21, m22],
        ])
    }

    pub fn from_array(data: [[f64; 3]; 3]) -> Self {
        Self(data)
    }

    pub fn as_array(&self) -> &[[f64; 3]; 3] {
        &self.0
    }

    pub fn identity() -> Self {
        Self([
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ])
    }

    pub fn zero() -> Self {
        Self([
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
        ])
    }

    pub fn from_diagonal(diagonal: Vector3) -> Self {
        Self([
            [diagonal.0, 0.0, 0.0],
            [0.0, diagonal.1, 0.0],
            [0.0, 0.0, diagonal.2],
        ])
    }

    pub fn from_diagonal_values(x: f64, y: f64, z: f64) -> Self {
        Self([
            [x, 0.0, 0.0],
            [0.0, y, 0.0],
            [0.0, 0.0, z],
        ])
    }

    pub fn from_rows(r0: Vector3, r1: Vector3, r2: Vector3) -> Self {
        Self([
            [r0.0, r0.1, r0.2],
            [r1.0, r1.1, r1.2],
            [r2.0, r2.1, r2.2],
        ])
    }

    pub fn from_cols(c0: Vector3, c1: Vector3, c2: Vector3) -> Self {
        Self([
            [c0.0, c1.0, c2.0],
            [c0.1, c1.1, c2.1],
            [c0.2, c1.2, c2.2],
        ])
    }

    pub fn row(&self, index: usize) -> Vector3 {
        Vector3::new(self.0[index][0], self.0[index][1], self.0[index][2])
    }

    pub fn col(&self, index: usize) -> Vector3 {
        Vector3::new(self.0[0][index], self.0[1][index], self.0[2][index])
    }

    pub fn add(self, rhs: Self) -> Self {
        Self([
            [self.0[0][0] + rhs.0[0][0], self.0[0][1] + rhs.0[0][1], self.0[0][2] + rhs.0[0][2]],
            [self.0[1][0] + rhs.0[1][0], self.0[1][1] + rhs.0[1][1], self.0[1][2] + rhs.0[1][2]],
            [self.0[2][0] + rhs.0[2][0], self.0[2][1] + rhs.0[2][1], self.0[2][2] + rhs.0[2][2]],
        ])
    }

    pub fn sub(self, rhs: Self) -> Self {
        Self([
            [self.0[0][0] - rhs.0[0][0], self.0[0][1] - rhs.0[0][1], self.0[0][2] - rhs.0[0][2]],
            [self.0[1][0] - rhs.0[1][0], self.0[1][1] - rhs.0[1][1], self.0[1][2] - rhs.0[1][2]],
            [self.0[2][0] - rhs.0[2][0], self.0[2][1] - rhs.0[2][1], self.0[2][2] - rhs.0[2][2]],
        ])
    }

    pub fn mul_scalar(self, scalar: f64) -> Self {
        Self([
            [self.0[0][0] * scalar, self.0[0][1] * scalar, self.0[0][2] * scalar],
            [self.0[1][0] * scalar, self.0[1][1] * scalar, self.0[1][2] * scalar],
            [self.0[2][0] * scalar, self.0[2][1] * scalar, self.0[2][2] * scalar],
        ])
    }

    pub fn transpose(self) -> Self {
        Self([
            [self.0[0][0], self.0[1][0], self.0[2][0]],
            [self.0[0][1], self.0[1][1], self.0[2][1]],
            [self.0[0][2], self.0[1][2], self.0[2][2]],
        ])
    }

    pub fn determinant(&self) -> f64 {
        let m = &self.0;
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) -
            m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]) +
            m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    }

    pub fn inverse(&self) -> Option<Self> {
        let det = self.determinant();
        if !det.is_finite() || det.abs() < 1e-15 {
            return None;
        }

        let m = &self.0;
        let c00 = m[1][1] * m[2][2] - m[1][2] * m[2][1];
        let c01 = -(m[1][0] * m[2][2] - m[1][2] * m[2][0]);
        let c02 = m[1][0] * m[2][1] - m[1][1] * m[2][0];

        let c10 = -(m[0][1] * m[2][2] - m[0][2] * m[2][1]);
        let c11 = m[0][0] * m[2][2] - m[0][2] * m[2][0];
        let c12 = -(m[0][0] * m[2][1] - m[0][1] * m[2][0]);

        let c20 = m[0][1] * m[1][2] - m[0][2] * m[1][1];
        let c21 = -(m[0][0] * m[1][2] - m[0][2] * m[1][0]);
        let c22 = m[0][0] * m[1][1] - m[0][1] * m[1][0];

        let inv_det = 1.0 / det;
        Some(
            Self([
                [c00 * inv_det, c10 * inv_det, c20 * inv_det],
                [c01 * inv_det, c11 * inv_det, c21 * inv_det],
                [c02 * inv_det, c12 * inv_det, c22 * inv_det],
            ])
        )
    }

    pub fn multiply(self, rhs: Self) -> Self {
        let mut res = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                res[i][j] =
                    self.0[i][0] * rhs.0[0][j] +
                    self.0[i][1] * rhs.0[1][j] +
                    self.0[i][2] * rhs.0[2][j];
            }
        }
        Self(res)
    }

    pub fn multiply_vector(self, rhs: Vector3) -> Vector3 {
        Vector3::new(
            self.0[0][0] * rhs.0 + self.0[0][1] * rhs.1 + self.0[0][2] * rhs.2,
            self.0[1][0] * rhs.0 + self.0[1][1] * rhs.1 + self.0[1][2] * rhs.2,
            self.0[2][0] * rhs.0 + self.0[2][1] * rhs.1 + self.0[2][2] * rhs.2
        )
    }
}

impl std::ops::Add for Matrix3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        self.add(rhs)
    }
}

impl std::ops::Add<&Matrix3> for Matrix3 {
    type Output = Self;
    fn add(self, rhs: &Matrix3) -> Self::Output {
        self.add(*rhs)
    }
}

impl std::ops::Add<Matrix3> for &Matrix3 {
    type Output = Matrix3;
    fn add(self, rhs: Matrix3) -> Self::Output {
        (*self).add(rhs)
    }
}

impl std::ops::Add<&Matrix3> for &Matrix3 {
    type Output = Matrix3;
    fn add(self, rhs: &Matrix3) -> Self::Output {
        (*self).add(*rhs)
    }
}

impl std::ops::Sub for Matrix3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        self.sub(rhs)
    }
}

impl std::ops::Sub<&Matrix3> for Matrix3 {
    type Output = Self;
    fn sub(self, rhs: &Matrix3) -> Self::Output {
        self.sub(*rhs)
    }
}

impl std::ops::Sub<Matrix3> for &Matrix3 {
    type Output = Matrix3;
    fn sub(self, rhs: Matrix3) -> Self::Output {
        (*self).sub(rhs)
    }
}

impl std::ops::Sub<&Matrix3> for &Matrix3 {
    type Output = Matrix3;
    fn sub(self, rhs: &Matrix3) -> Self::Output {
        (*self).sub(*rhs)
    }
}

impl std::ops::Neg for Matrix3 {
    type Output = Self;
    fn neg(self) -> Self::Output {
        self.mul_scalar(-1.0)
    }
}

impl std::ops::Neg for &Matrix3 {
    type Output = Matrix3;
    fn neg(self) -> Self::Output {
        (*self).mul_scalar(-1.0)
    }
}

impl std::ops::Mul<f64> for Matrix3 {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self::Output {
        self.mul_scalar(rhs)
    }
}

impl std::ops::Mul<f64> for &Matrix3 {
    type Output = Matrix3;
    fn mul(self, rhs: f64) -> Self::Output {
        (*self).mul_scalar(rhs)
    }
}

impl std::ops::Mul<Matrix3> for f64 {
    type Output = Matrix3;
    fn mul(self, rhs: Matrix3) -> Self::Output {
        rhs.mul_scalar(self)
    }
}

impl std::ops::Mul<&Matrix3> for f64 {
    type Output = Matrix3;
    fn mul(self, rhs: &Matrix3) -> Self::Output {
        (*rhs).mul_scalar(self)
    }
}

impl std::ops::Div<f64> for Matrix3 {
    type Output = Self;
    fn div(self, rhs: f64) -> Self::Output {
        self.mul_scalar(1.0 / rhs)
    }
}

impl std::ops::Div<f64> for &Matrix3 {
    type Output = Matrix3;
    fn div(self, rhs: f64) -> Self::Output {
        (*self).mul_scalar(1.0 / rhs)
    }
}

impl std::ops::Mul<Matrix3> for Matrix3 {
    type Output = Self;
    fn mul(self, rhs: Matrix3) -> Self::Output {
        self.multiply(rhs)
    }
}

impl std::ops::Mul<&Matrix3> for Matrix3 {
    type Output = Self;
    fn mul(self, rhs: &Matrix3) -> Self::Output {
        self.multiply(*rhs)
    }
}

impl std::ops::Mul<Matrix3> for &Matrix3 {
    type Output = Matrix3;
    fn mul(self, rhs: Matrix3) -> Self::Output {
        (*self).multiply(rhs)
    }
}

impl std::ops::Mul<&Matrix3> for &Matrix3 {
    type Output = Matrix3;
    fn mul(self, rhs: &Matrix3) -> Self::Output {
        (*self).multiply(*rhs)
    }
}

impl std::ops::Mul<Vector3> for Matrix3 {
    type Output = Vector3;
    fn mul(self, rhs: Vector3) -> Self::Output {
        self.multiply_vector(rhs)
    }
}

impl std::ops::Mul<&Vector3> for Matrix3 {
    type Output = Vector3;
    fn mul(self, rhs: &Vector3) -> Self::Output {
        self.multiply_vector(*rhs)
    }
}

impl std::ops::Mul<Vector3> for &Matrix3 {
    type Output = Vector3;
    fn mul(self, rhs: Vector3) -> Self::Output {
        (*self).multiply_vector(rhs)
    }
}

impl std::ops::Mul<&Vector3> for &Matrix3 {
    type Output = Vector3;
    fn mul(self, rhs: &Vector3) -> Self::Output {
        (*self).multiply_vector(*rhs)
    }
}

impl std::ops::Index<usize> for Matrix3 {
    type Output = [f64; 3];
    fn index(&self, row: usize) -> &Self::Output {
        &self.0[row]
    }
}

impl std::ops::IndexMut<usize> for Matrix3 {
    fn index_mut(&mut self, row: usize) -> &mut Self::Output {
        &mut self.0[row]
    }
}

impl std::ops::Index<(usize, usize)> for Matrix3 {
    type Output = f64;
    fn index(&self, (row, col): (usize, usize)) -> &Self::Output {
        &self.0[row][col]
    }
}

impl std::ops::IndexMut<(usize, usize)> for Matrix3 {
    fn index_mut(&mut self, (row, col): (usize, usize)) -> &mut Self::Output {
        &mut self.0[row][col]
    }
}
