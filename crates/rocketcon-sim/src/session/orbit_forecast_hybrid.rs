use super::orbit_forecast::{ForecastResult, sample_patched_forecast};
use super::orbit_forecast_hybrid_window::sample_restricted_forecast;
use super::orbit_preview_horizon::MAX_BOUND_HORIZON;
use crate::{RocketError, RocketResult};
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::units::Duration;
use rocketcon_app::orbital::hybrid_regime::{GravityRegime, RegimePolicy, choose_regime};
use rocketcon_core::domain::VehiclePhysicalState;

pub(super) fn sample_hybrid_forecast_cancelled(
    catalog: &EphemerisCatalog,
    initial: &VehiclePhysicalState,
    horizon: f64,
    cancelled: &mut impl FnMut() -> bool,
    progress: &mut impl FnMut(&ForecastResult),
) -> RocketResult<ForecastResult> {
    if cancelled() {
        return Err(RocketError::Generic("forecast cancelled".into()));
    }
    let regime = choose_regime(
        catalog,
        initial,
        Duration::new(horizon.min(MAX_BOUND_HORIZON)),
        false,
        RegimePolicy::forecast(),
    )
    .map_err(|error| RocketError::Generic(error.to_string()))?;
    match regime {
        GravityRegime::Kepler | GravityRegime::Specialized => {
            let forecast = sample_patched_forecast(
                catalog,
                initial.captured_total_epoch(),
                initial.reference_body_id(),
                initial.position(),
                initial.velocity(),
                horizon,
            )?;
            if cancelled() {
                Err(RocketError::Generic("forecast cancelled".into()))
            } else {
                Ok(forecast)
            }
        }
        GravityRegime::Restricted { sources } => sample_restricted_forecast(
            catalog,
            initial.captured_total_epoch(),
            initial.reference_body_id(),
            initial.position(),
            initial.velocity(),
            horizon,
            &sources,
            cancelled,
            progress,
        ),
    }
}
