use crate::units::angle::Angle;
use crate::units::angular_acceleration::AngularAcceleration;
use crate::units::angular_momentum::AngularMomentum;
use crate::units::angular_velocity::AngularVelocity;
use crate::units::duration::Duration;
use crate::units::energy::Energy;
use crate::units::force::Force;
use crate::units::impulse::Impulse;
use crate::units::length::Length;
use crate::units::luminosity::Luminosity;
use crate::units::mass::Mass;
use crate::units::mass_rate::MassRate;
use crate::units::moment_of_inertia::MomentOfInertia;
use crate::units::position::Position;
use crate::units::scalar_macro::define_rate_of_change;
use crate::units::specific_energy::SpecificEnergy;
use crate::units::speed::Speed;
use crate::units::torque::Torque;
use crate::units::velocity::Velocity;

define_rate_of_change!(Speed, Duration, Length);
define_rate_of_change!(AngularVelocity, Duration, Angle);
define_rate_of_change!(MassRate, Duration, Mass);
define_rate_of_change!(AngularAcceleration, Duration, AngularVelocity);
define_rate_of_change!(Torque, Duration, AngularMomentum);
define_rate_of_change!(Force, Duration, Impulse);
define_rate_of_change!(Luminosity, Duration, Energy);

impl std::ops::Mul<Duration> for Velocity {
    type Output = Position;
    fn mul(self, rhs: Duration) -> Self::Output {
        Position::from_raw(self.raw() * rhs.value())
    }
}

impl std::ops::Mul<AngularAcceleration> for MomentOfInertia {
    type Output = Torque;
    fn mul(self, rhs: AngularAcceleration) -> Self::Output {
        Torque::new(self.value() * rhs.value())
    }
}

impl std::ops::Mul<Mass> for SpecificEnergy {
    type Output = Energy;
    fn mul(self, rhs: Mass) -> Self::Output {
        Energy::new(self.value() * rhs.value())
    }
}