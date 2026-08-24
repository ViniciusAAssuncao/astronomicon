macro_rules! define_vector_quantity {
    ($name:ident, $magnitude:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
        pub struct $name(pub(crate) crate::units::vector3::Vector3);

        impl $name {
            pub fn from_components(x: f64, y: f64, z: f64) -> Self {
                Self(crate::units::vector3::Vector3::new(x, y, z))
            }

            pub fn from_raw(raw: crate::units::vector3::Vector3) -> Self {
                Self(raw)
            }

            pub fn raw(self) -> crate::units::vector3::Vector3 {
                self.0
            }

            pub fn zero() -> Self {
                Self(crate::units::vector3::Vector3::zero())
            }

            pub fn magnitude(&self) -> $magnitude {
                $magnitude::new(self.0.magnitude())
            }
        }

        impl std::ops::Add for $name {
            type Output = Self;
            fn add(self, rhs: Self) -> Self::Output {
                Self(self.0 + rhs.0)
            }
        }

        impl std::ops::Sub for $name {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self::Output {
                Self(self.0 - rhs.0)
            }
        }

        impl std::ops::Neg for $name {
            type Output = Self;
            fn neg(self) -> Self::Output {
                Self(-self.0)
            }
        }

        impl std::ops::Mul<f64> for $name {
            type Output = Self;
            fn mul(self, rhs: f64) -> Self::Output {
                Self(self.0 * rhs)
            }
        }

        impl std::ops::Div<f64> for $name {
            type Output = Self;
            fn div(self, rhs: f64) -> Self::Output {
                Self(self.0 / rhs)
            }
        }
    };
}

pub(crate) use define_vector_quantity;
