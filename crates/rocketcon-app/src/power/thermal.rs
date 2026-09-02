use crate::error::RocketResult;
use astronomicon_core::units::{Duration, Luminosity, Position, Temperature};
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::ComponentDetails;
use rocketcon_core::environment::EnvironmentSnapshot;
use rocketcon_core::math::thermal_budget::{
    effective_ga_product, vehicle_equilibrium_temperature,
};
use rocketcon_db::repositories::vehicle as vehicle_repository;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VehicleThermalBudget {
    pub total_internal_heat_generation: Luminosity,
    pub effective_radiator_ga_product: f64,
    pub equilibrium_temperature: Temperature,
}

impl VehicleThermalBudget {
    pub fn new(
        total_internal_heat_generation: Luminosity,
        effective_radiator_ga_product: f64,
        equilibrium_temperature: Temperature,
    ) -> Self {
        Self {
            total_internal_heat_generation,
            effective_radiator_ga_product,
            equilibrium_temperature,
        }
    }
}

pub async fn resolve_vehicle_thermal_budget(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    _universe_epoch: Duration,
    _at_epoch: Duration,
    _environment: &EnvironmentSnapshot,
    _vehicle_position: Position,
    total_internal_waste_heat: Luminosity,
) -> RocketResult<VehicleThermalBudget> {
    let components = vehicle_repository::list_components_for_vehicle(pool, &vehicle_id).await?;

    let mut radiators = Vec::new();
    for (_, record) in components {
        if let ComponentDetails::Radiator(spec) = record.details() {
            radiators.push((spec.radiating_area_m2(), spec.emissivity()));
        }
    }

    let effective_ga = effective_ga_product(&radiators);
    let eq_temp = vehicle_equilibrium_temperature(total_internal_waste_heat, effective_ga);

    Ok(VehicleThermalBudget {
        total_internal_heat_generation: total_internal_waste_heat,
        effective_radiator_ga_product: effective_ga,
        equilibrium_temperature: eq_temp,
    })
}
