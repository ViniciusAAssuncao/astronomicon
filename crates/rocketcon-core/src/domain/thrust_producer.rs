use astronomicon_core::units::constants::STANDARD_GRAVITY;
use astronomicon_core::units::{Duration, Force, MassRate, Speed};

pub trait ThrustProducer {
    fn specific_impulse_vacuum(&self) -> Duration;
    fn max_thrust(&self) -> Force;

    fn effective_exhaust_velocity_vacuum(&self) -> Speed {
        Speed::new(self.specific_impulse_vacuum().value() * STANDARD_GRAVITY)
    }

    fn propellant_mass_flow_rate_at_max_thrust(&self) -> MassRate {
        let v_e = self.effective_exhaust_velocity_vacuum().value();
        if v_e <= 0.0 || !v_e.is_finite() {
            MassRate::new(0.0)
        } else {
            MassRate::new(self.max_thrust().value() / v_e)
        }
    }
}
