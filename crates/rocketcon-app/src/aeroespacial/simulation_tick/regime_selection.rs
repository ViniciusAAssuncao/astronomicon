use crate::error::RocketResult;
use crate::orbital::hybrid_regime::{GravityRegime, RegimePolicy, choose_regime};
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::units::Duration;
use rocketcon_core::domain::VehiclePhysicalState;

pub(super) struct TickGravitySelection {
    pub(super) regime: GravityRegime,
    pub(super) use_force_integration: bool,
}

impl TickGravitySelection {
    pub(super) fn nbody_coasting(&self) -> bool {
        !self.use_force_integration && self.regime.sources().is_some()
    }

    pub(super) fn nbody_powered(&self) -> bool {
        self.use_force_integration && self.regime.sources().is_some()
    }
}

pub(super) fn select_tick_gravity(
    catalog: &EphemerisCatalog,
    state: &VehiclePhysicalState,
    dt: Duration,
    propelling: bool,
    drag: bool,
    contact: bool,
) -> RocketResult<TickGravitySelection> {
    let regime = if drag || contact {
        GravityRegime::Specialized
    } else {
        choose_regime(catalog, state, dt, propelling, RegimePolicy::flight())?
    };
    Ok(TickGravitySelection {
        regime,
        use_force_integration: propelling || drag || contact,
    })
}
