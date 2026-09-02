use crate::error::RocketResult;
use astronomicon_app::climate::resolve_irradiance_at_position;
use astronomicon_core::math::eclipse::is_in_cylindrical_shadow;
use astronomicon_core::units::{Duration, Length, Luminosity, Position, Quaternion, Vector3};
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::{ComponentDetails, ComponentRecord, VehicleComponentEntry};
use rocketcon_core::environment::EnvironmentSnapshot;
use rocketcon_core::math::{
    nuclear_decay::{rtg_electrical_power, rtg_thermal_power},
    nuclear_reactor::{
        reactor_electrical_power, reactor_initial_fuel_energy, reactor_thermal_power_at_throttle,
        reactor_waste_heat,
    },
    power_budget::ComponentPowerContribution,
    solar_power_generation::{
        solar_incidence_factor, solar_panel_electrical_output, solar_panel_waste_heat,
    },
};
use rocketcon_db::repositories::{energy_reservoir_repository, operational_state_repository};

pub async fn resolve_component_generation(
    pool: &SqlitePool,
    entry: &VehicleComponentEntry,
    record: &ComponentRecord,
    environment: &EnvironmentSnapshot,
    vehicle_position: Position,
    vehicle_orientation: Quaternion,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<ComponentPowerContribution> {
    match record.details() {
        ComponentDetails::Rtg(spec) => {
            let operational_state =
                operational_state_repository::get_by_vehicle_component_id(pool, &entry.id())
                    .await?;
            let load_fraction = operational_state.map(|s| s.load_fraction()).unwrap_or(1.0);

            let total_epoch = universe_epoch + at_epoch;
            let elapsed_val =
                (total_epoch.value() - spec.fuel_loaded_universe_epoch().value()).max(0.0);
            let elapsed = Duration::new(elapsed_val);

            let p_th = rtg_thermal_power(spec.radioisotope(), spec.fuel_mass(), elapsed);
            let p_el_nominal = rtg_electrical_power(
                spec.radioisotope(),
                spec.fuel_mass(),
                spec.conversion_efficiency(),
                elapsed,
            );

            let is_connected = load_fraction >= 1e-4;
            let electrical_generation = if is_connected {
                Luminosity::new(p_el_nominal.value() * load_fraction)
            } else {
                Luminosity::new(0.0)
            };

            let waste_heat =
                Luminosity::new((p_th.value() - electrical_generation.value()).max(0.0));

            Ok(ComponentPowerContribution::new(
                electrical_generation,
                Luminosity::new(0.0),
                waste_heat,
            ))
        }
        ComponentDetails::NuclearReactor(spec) => {
            let reservoir_state =
                energy_reservoir_repository::get_by_vehicle_component_id(pool, &entry.id())
                    .await?;
            let stored_energy = match reservoir_state {
                Some(state) => state.stored_energy(),
                None => reactor_initial_fuel_energy(spec.fuel_type(), spec.fuel_mass()),
            };

            let operational_state =
                operational_state_repository::get_by_vehicle_component_id(pool, &entry.id())
                    .await?;
            let load_fraction = operational_state.map(|s| s.load_fraction()).unwrap_or(1.0);

            if stored_energy.value() <= 0.0 {
                return Ok(ComponentPowerContribution::new(
                    Luminosity::new(0.0),
                    Luminosity::new(0.0),
                    Luminosity::new(0.0),
                ));
            }

            let p_th = reactor_thermal_power_at_throttle(
                spec.max_thermal_power(),
                load_fraction,
                spec.min_throttle_fraction(),
            );
            let p_el = reactor_electrical_power(p_th, spec.conversion_efficiency());
            let p_waste = reactor_waste_heat(p_th, p_el);

            Ok(ComponentPowerContribution::new(
                p_el,
                Luminosity::new(0.0),
                p_waste,
            ))
        }
        ComponentDetails::SolarPanel(spec) => {
            let irradiance = resolve_irradiance_at_position(
                pool,
                &environment.star,
                universe_epoch,
                at_epoch,
                vehicle_position,
            )
            .await?;

            let planet_radius = environment
                .planet
                .equatorial_radius()
                .unwrap_or_else(|| Length::new(6371e3));

            let is_eclipsed = is_in_cylindrical_shadow(
                vehicle_position,
                environment.star_position,
                environment.planet_position,
                planet_radius,
            );

            let operational_state =
                operational_state_repository::get_by_vehicle_component_id(pool, &entry.id())
                    .await?;
            let load_fraction = operational_state.map(|s| s.load_fraction()).unwrap_or(1.0);

            let effective_area = spec.surface_area_m2() * load_fraction;

            let local_normal = entry.actuation_axis().unwrap_or(Vector3::new(0.0, 0.0, 1.0));
            let panel_normal_world = vehicle_orientation.rotate_vector(local_normal);
            let sun_direction_world =
                (environment.star_position.raw() - vehicle_position.raw()).normalized();

            let incidence = solar_incidence_factor(
                panel_normal_world,
                sun_direction_world,
                spec.is_sun_tracking(),
            );

            let p_el = solar_panel_electrical_output(
                irradiance,
                effective_area,
                spec.conversion_efficiency(),
                incidence,
                is_eclipsed,
            );
            let p_waste = solar_panel_waste_heat(
                irradiance,
                effective_area,
                spec.effective_solar_absorptivity(),
                spec.conversion_efficiency(),
                incidence,
                is_eclipsed,
            );

            Ok(ComponentPowerContribution::new(
                p_el,
                Luminosity::new(0.0),
                p_waste,
            ))
        }
        _ => Ok(ComponentPowerContribution::new(
            Luminosity::new(0.0),
            Luminosity::new(0.0),
            Luminosity::new(0.0),
        )),
    }
}