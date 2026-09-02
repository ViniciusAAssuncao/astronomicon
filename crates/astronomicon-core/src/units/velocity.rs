use crate::units::speed::Speed;
use crate::units::vector_macro::define_vector_quantity;

define_vector_quantity!(Velocity, Speed);

pub type VelocityVector = Velocity;