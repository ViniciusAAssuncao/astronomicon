use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vector3(pub f64, pub f64, pub f64);

impl Vector3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self(x, y, z)
    }

    pub fn zero() -> Self {
        Self(0.0, 0.0, 0.0)
    }

    pub fn dot(&self, other: &Self) -> f64 {
        self.0 * other.0 + self.1 * other.1 + self.2 * other.2
    }

    pub fn cross(&self, other: &Self) -> Self {
        Self(
            self.1 * other.2 - self.2 * other.1,
            self.2 * other.0 - self.0 * other.2,
            self.0 * other.1 - self.1 * other.0,
        )
    }

    pub fn magnitude(&self) -> f64 {
        self.dot(self).sqrt()
    }

    pub fn normalized(&self) -> Self {
        let mag = self.magnitude();
        if mag < 1e-12 {
            Self::zero()
        } else {
            *self / mag
        }
    }

    pub fn any_perpendicular(&self) -> Self {
        let n = self.normalized();
        let arbitrary = if n.0.abs() < 0.9 {
            Self::new(1.0, 0.0, 0.0)
        } else {
            Self::new(0.0, 1.0, 0.0)
        };
        n.cross(&arbitrary).normalized()
    }

    pub fn rotate_about_axis(&self, axis: Vector3, angle_rad: f64) -> Self {
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();
        let k = axis;
        let v = *self;

        let term1 = v * cos_a;
        let term2 = k.cross(&v) * sin_a;
        let term3 = k * (k.dot(&v) * (1.0 - cos_a));

        term1 + term2 + term3
    }

    pub fn rotate_about_x(&self, angle_rad: f64) -> Self {
        self.rotate_about_axis(Vector3::new(1.0, 0.0, 0.0), angle_rad)
    }

    pub fn rotate_about_y(&self, angle_rad: f64) -> Self {
        self.rotate_about_axis(Vector3::new(0.0, 1.0, 0.0), angle_rad)
    }

    pub fn rotate_about_z(&self, angle_rad: f64) -> Self {
        self.rotate_about_axis(Vector3::new(0.0, 0.0, 1.0), angle_rad)
    }
}

impl std::ops::Add for Vector3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0, self.1 + rhs.1, self.2 + rhs.2)
    }
}

impl std::ops::Sub for Vector3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0, self.1 - rhs.1, self.2 - rhs.2)
    }
}

impl std::ops::Neg for Vector3 {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self(-self.0, -self.1, -self.2)
    }
}

impl std::ops::Mul<f64> for Vector3 {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self::Output {
        Self(self.0 * rhs, self.1 * rhs, self.2 * rhs)
    }
}

impl std::ops::Div<f64> for Vector3 {
    type Output = Self;
    fn div(self, rhs: f64) -> Self::Output {
        Self(self.0 / rhs, self.1 / rhs, self.2 / rhs)
    }
}