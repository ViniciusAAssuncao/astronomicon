use crate::units::angle::Angle;
use crate::units::angular_velocity::AngularVelocity;
use crate::units::duration::Duration;
use crate::units::length::Length;
use crate::units::position::Position;
use crate::units::scalar_macro::define_rate_of_change;
use crate::units::speed::Speed;
use crate::units::velocity::Velocity;

define_rate_of_change!(Speed, Duration, Length);
define_rate_of_change!(AngularVelocity, Duration, Angle);

impl std::ops::Mul<Duration> for Velocity {
    type Output = Position;
    fn mul(self, rhs: Duration) -> Self::Output {
        Position::from_raw(self.raw() * rhs.value())
    }
}