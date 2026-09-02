use astronomicon_core::units::Luminosity;
use rocketcon_core::domain::{ComponentOperationalState, ComponentRecord, VehicleComponentEntry};
use rocketcon_core::math::power_budget::{
    component_consumption_waste_heat, component_power_consumption, ComponentPowerContribution,
};

pub fn resolve_component_consumption(
    _entry: &VehicleComponentEntry,
    record: &ComponentRecord,
    operational_state: Option<ComponentOperationalState>,
) -> ComponentPowerContribution {
    let rated_power = Luminosity::new(record.component().power_consumption_w());
    let load_fraction = operational_state.map(|s| s.load_fraction()).unwrap_or(1.0);
    let consumed_power = component_power_consumption(rated_power, load_fraction);
    let waste_heat = component_consumption_waste_heat(consumed_power);

    ComponentPowerContribution::new(Luminosity::new(0.0), consumed_power, waste_heat)
}
