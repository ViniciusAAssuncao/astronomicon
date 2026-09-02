use crate::units::mass::Mass;
use crate::units::matrix3::Matrix3;
use crate::units::vector3::Vector3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InertiaTensor(pub(crate) Matrix3);

impl InertiaTensor {
    pub fn new(matrix: Matrix3) -> Self {
        Self(matrix)
    }

    pub fn from_raw(matrix: Matrix3) -> Self {
        Self(matrix)
    }

    pub fn raw(self) -> Matrix3 {
        self.0
    }

    pub fn matrix(&self) -> Matrix3 {
        self.0
    }

    pub fn zero() -> Self {
        Self(Matrix3::zero())
    }

    pub fn principal_diagonal(ixx: f64, iyy: f64, izz: f64) -> Self {
        let x = if ixx.is_finite() && ixx >= 0.0 { ixx } else { 0.0 };
        let y = if iyy.is_finite() && iyy >= 0.0 { iyy } else { 0.0 };
        let z = if izz.is_finite() && izz >= 0.0 { izz } else { 0.0 };
        Self(Matrix3::new(
            x, 0.0, 0.0,
            0.0, y, 0.0,
            0.0, 0.0, z,
        ))
    }

    pub fn parallel_axis_shift(&self, mass: Mass, offset: Vector3) -> Self {
        let m = mass.value();
        if m <= 0.0 || !m.is_finite() {
            return *self;
        }

        let dx = offset.0;
        let dy = offset.1;
        let dz = offset.2;

        if !dx.is_finite() || !dy.is_finite() || !dz.is_finite() {
            return *self;
        }

        let shift = Matrix3::new(
            m * (dy * dy + dz * dz),
            -m * dx * dy,
            -m * dx * dz,
            -m * dy * dx,
            m * (dx * dx + dz * dz),
            -m * dy * dz,
            -m * dz * dx,
            -m * dz * dy,
            m * (dx * dx + dy * dy),
        );

        Self(self.0.add(shift))
    }

    pub fn rotate_by(&self, rotation_matrix: &Matrix3) -> Self {
        let r = *rotation_matrix;
        let r_t = r.transpose();
        let rotated = r.multiply(self.0).multiply(r_t);
        Self(rotated)
    }

    pub fn add(&self, other: impl std::borrow::Borrow<Self>) -> Self {
        Self(self.0.add(other.borrow().0))
    }

    pub fn inverse(&self) -> Option<Self> {
        self.0.inverse().map(Self)
    }

    pub fn ixx(&self) -> f64 {
        self.0.0[0][0]
    }

    pub fn iyy(&self) -> f64 {
        self.0.0[1][1]
    }

    pub fn izz(&self) -> f64 {
        self.0.0[2][2]
    }

    pub fn ixy(&self) -> f64 {
        self.0.0[0][1]
    }

    pub fn ixz(&self) -> f64 {
        self.0.0[0][2]
    }

    pub fn iyz(&self) -> f64 {
        self.0.0[1][2]
    }
}

impl std::ops::Add for InertiaTensor {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.add(rhs.0))
    }
}

impl std::ops::Add<&InertiaTensor> for InertiaTensor {
    type Output = Self;
    fn add(self, rhs: &InertiaTensor) -> Self::Output {
        Self(self.0.add(rhs.0))
    }
}

impl std::ops::Add<InertiaTensor> for &InertiaTensor {
    type Output = InertiaTensor;
    fn add(self, rhs: InertiaTensor) -> Self::Output {
        InertiaTensor(self.0.add(rhs.0))
    }
}

impl std::ops::Add<&InertiaTensor> for &InertiaTensor {
    type Output = InertiaTensor;
    fn add(self, rhs: &InertiaTensor) -> Self::Output {
        InertiaTensor(self.0.add(rhs.0))
    }
}