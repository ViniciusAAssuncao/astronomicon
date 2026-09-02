use crate::aeroespacial::resolve_vehicle_assembly;
use crate::error::{ RocketError, RocketResult };
use astronomicon_core::domain::Planet;
use astronomicon_core::math::gravity::{ gravitational_parameter, surface_gravity };
use astronomicon_core::units::{ Duration, Length, Mass, MassRate, Speed };
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::planet_repository;
use rocketcon_core::math::{
    burn_time,
    combined_effective_exhaust_velocity,
    mass_ratio,
    propellant_mass_fraction,
    thrust_to_weight_ratio,
    tsiolkovsky_delta_v,
};
use serde::{ Deserialize, Serialize };
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehiclePerformanceSummary {
    pub wet_mass: Mass,
    pub dry_mass: Mass,
    pub mass_ratio: f64,
    pub propellant_mass_fraction: f64,
    pub combined_effective_exhaust_velocity: Speed,
    pub delta_v: Speed,
    pub limiting_burn_time: Duration,
    pub thrust_to_weight_ratio: f64,
}

impl VehiclePerformanceSummary {
    pub fn new(
        wet_mass: Mass,
        dry_mass: Mass,
        mass_ratio: f64,
        propellant_mass_fraction: f64,
        combined_effective_exhaust_velocity: Speed,
        delta_v: Speed,
        limiting_burn_time: Duration,
        thrust_to_weight_ratio: f64
    ) -> Self {
        Self {
            wet_mass,
            dry_mass,
            mass_ratio,
            propellant_mass_fraction,
            combined_effective_exhaust_velocity,
            delta_v,
            limiting_burn_time,
            thrust_to_weight_ratio,
        }
    }
}

pub async fn resolve_vehicle_performance_summary(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    reference_planet_id: Uuid,
    _universe_epoch: Duration,
    _at_epoch: Duration
) -> RocketResult<VehiclePerformanceSummary> {
    let assembly = resolve_vehicle_assembly(pool, vehicle_id).await?;

    let dry_mass = assembly.total_dry_mass;
    let mut total_propellant_mass = 0.0;
    for cap in assembly.total_propellant_capacity_by_propellant.values() {
        total_propellant_mass += cap.value();
    }
    let wet_mass = Mass::new(dry_mass.value() + total_propellant_mass);

    let mut total_flow = 0.0;
    for flow in assembly.total_mass_flow_rate_by_propellant.values() {
        total_flow += flow.value();
    }
    let total_mass_flow_rate = MassRate::new(total_flow);

    let combined_ve = combined_effective_exhaust_velocity(
        assembly.total_max_thrust,
        total_mass_flow_rate
    );
    let delta_v = tsiolkovsky_delta_v(combined_ve, wet_mass, dry_mass);
    let m_ratio = mass_ratio(wet_mass, dry_mass);
    let prop_frac = propellant_mass_fraction(wet_mass, dry_mass);

    let mut limiting_burn_time = Duration::new(0.0);
    let mut min_burn_time: Option<f64> = None;

    for (prop_id, flow_rate) in &assembly.total_mass_flow_rate_by_propellant {
        if flow_rate.value() > 0.0 {
            let capacity = assembly.total_propellant_capacity_by_propellant
                .get(prop_id)
                .copied()
                .unwrap_or_else(|| Mass::new(0.0));
            let tb = burn_time(capacity, *flow_rate).value();
            min_burn_time = Some(match min_burn_time {
                Some(current_min) => current_min.min(tb),
                None => tb,
            });
        }
    }

    if let Some(tb) = min_burn_time {
        limiting_burn_time = Duration::new(tb);
    }

    let planet_row = planet_repository
        ::get_by_id(pool, &reference_planet_id).await?
        .ok_or_else(||
            RocketError::Generic(format!("planet '{}' not found", reference_planet_id))
        )?;

    let planet = Planet::try_from(planet_row)?;
    let radius = planet.equatorial_radius().unwrap_or_else(|| Length::new(6371e3));
    let mu = gravitational_parameter(planet.mass());
    let g = surface_gravity(mu, radius);
    let twr = thrust_to_weight_ratio(assembly.total_max_thrust, wet_mass, g);

    Ok(
        VehiclePerformanceSummary::new(
            wet_mass,
            dry_mass,
            m_ratio,
            prop_frac,
            combined_ve,
            delta_v,
            limiting_burn_time,
            twr
        )
    )
}
