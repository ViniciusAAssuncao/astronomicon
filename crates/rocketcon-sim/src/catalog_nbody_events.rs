use astronomicon_core::units::Duration;
use rocketcon_core::math::orbital::n_body::dop853::Dop853Error;
pub use rocketcon_core::math::orbital::n_body::impact::{
    ImpactCandidate, ImpactEvent, first_impact_in_segment,
};

pub fn representable_epoch_step(epoch: Duration) -> Result<f64, Dop853Error> {
    let value = epoch.value().abs();
    if !value.is_finite() {
        return Err(Dop853Error::InvalidConfiguration);
    }
    let step = (value * f64::EPSILON).max(f64::EPSILON);
    if !step.is_finite() || step <= 0.0 {
        return Err(Dop853Error::InvalidConfiguration);
    }
    Ok(step)
}