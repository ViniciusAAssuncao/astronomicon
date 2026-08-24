macro_rules! define_scalar_quantity {
    ($name:ident) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize,
        )]
        pub struct $name(pub(crate) f64);

        impl $name {
            pub fn new(value: f64) -> Self {
                Self(value)
            }

            pub fn value(self) -> f64 {
                self.0
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

        impl std::ops::Neg for $name {
            type Output = Self;
            fn neg(self) -> Self::Output {
                Self(-self.0)
            }
        }
    };
}

pub(crate) use define_scalar_quantity;

macro_rules! define_rate_of_change {
    ($rate:ident, $duration:ident, $accumulated:ident) => {
        impl std::ops::Mul<$duration> for $rate {
            type Output = $accumulated;
            fn mul(self, rhs: $duration) -> Self::Output {
                $accumulated::new(self.value() * rhs.value())
            }
        }
    };
}

pub(crate) use define_rate_of_change;
