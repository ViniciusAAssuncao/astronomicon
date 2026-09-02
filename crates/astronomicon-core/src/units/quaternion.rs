use crate::units::{Angle, AngularVelocityVector, Duration, Matrix3, Vector3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Quaternion {
    w: f64,
    x: f64,
    y: f64,
    z: f64,
}

impl Quaternion {
    pub fn new(w: f64, x: f64, y: f64, z: f64) -> Self {
        Self { w, x, y, z }
    }

    pub fn identity() -> Self {
        Self {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    pub fn from_axis_angle(axis: Vector3, angle: Angle) -> Self {
        let mag = axis.magnitude();
        if mag < 1e-12 {
            return Self::identity();
        }
        let half_angle = angle.value() * 0.5;
        let sin_half = half_angle.sin();
        let cos_half = half_angle.cos();
        let norm_axis = axis / mag;
        Self {
            w: cos_half,
            x: norm_axis.0 * sin_half,
            y: norm_axis.1 * sin_half,
            z: norm_axis.2 * sin_half,
        }
    }

    pub fn from_rotation_between(from: Vector3, to: Vector3) -> Self {
        if from.magnitude() < 1e-12 || to.magnitude() < 1e-12 {
            return Self::identity();
        }
        let u = from.normalized();
        let v = to.normalized();
        let dot = u.dot(&v);

        if dot >= 1.0 - 1e-12 {
            Self::identity()
        } else if dot <= -1.0 + 1e-12 {
            let axis = u.any_perpendicular();
            Self::new(0.0, axis.0, axis.1, axis.2)
        } else {
            let cross = u.cross(&v);
            Self::new(1.0 + dot, cross.0, cross.1, cross.2).normalized()
        }
    }

    pub fn w(&self) -> f64 {
        self.w
    }

    pub fn x(&self) -> f64 {
        self.x
    }

    pub fn y(&self) -> f64 {
        self.y
    }

    pub fn z(&self) -> f64 {
        self.z
    }

    pub fn norm_squared(&self) -> f64 {
        self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn norm(&self) -> f64 {
        self.norm_squared().sqrt()
    }

    pub fn is_normalized(&self, tolerance: f64) -> bool {
        (self.norm() - 1.0).abs() <= tolerance
    }

    pub fn normalized(&self) -> Self {
        let n = self.norm();
        if n < 1e-12 {
            Self::identity()
        } else {
            Self {
                w: self.w / n,
                x: self.x / n,
                y: self.y / n,
                z: self.z / n,
            }
        }
    }

    pub fn multiply(self, rhs: Self) -> Self {
        Self {
            w: self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
            x: self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            y: self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            z: self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
        }
    }

    pub fn rotate_vector(self, v: Vector3) -> Vector3 {
        let u = Vector3::new(self.x, self.y, self.z);
        let s = self.w;
        let t = u.cross(&v) * 2.0;
        v + t * s + u.cross(&t)
    }

    pub fn to_rotation_matrix(self) -> Matrix3 {
        let q = self.normalized();
        let w = q.w;
        let x = q.x;
        let y = q.y;
        let z = q.z;

        Matrix3::new(
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y - w * z),
            2.0 * (x * z + w * y),
            2.0 * (x * y + w * z),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z - w * x),
            2.0 * (x * z - w * y),
            2.0 * (y * z + w * x),
            1.0 - 2.0 * (x * x + y * y),
        )
    }

    pub fn conjugate(self) -> Self {
        Self {
            w: self.w,
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }

    pub fn inverse(self) -> Self {
        let n2 = self.norm_squared();
        if n2 < 1e-12 {
            Self::identity()
        } else {
            Self {
                w: self.w / n2,
                x: -self.x / n2,
                y: -self.y / n2,
                z: -self.z / n2,
            }
        }
    }

    pub fn derivative(self, angular_velocity: AngularVelocityVector) -> Self {
        let raw = angular_velocity.raw();
        let omega = Self::new(0.0, raw.0, raw.1, raw.2);
        let q_omega = self.multiply(omega);
        Self {
            w: 0.5 * q_omega.w,
            x: 0.5 * q_omega.x,
            y: 0.5 * q_omega.y,
            z: 0.5 * q_omega.z,
        }
    }

    pub fn integrate(self, angular_velocity: AngularVelocityVector, dt: Duration) -> Self {
        let dq = self.derivative(angular_velocity);
        self.add_scaled(dq, dt.value()).normalized()
    }

    pub fn add_scaled(self, other: Self, scale: f64) -> Self {
        Self {
            w: self.w + other.w * scale,
            x: self.x + other.x * scale,
            y: self.y + other.y * scale,
            z: self.z + other.z * scale,
        }
    }
}

impl std::ops::Mul for Quaternion {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        self.multiply(rhs)
    }
}
