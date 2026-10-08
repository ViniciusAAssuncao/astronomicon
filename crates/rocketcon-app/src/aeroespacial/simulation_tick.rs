use crate::aeroespacial::aerodynamics::resolve_vehicle_aerodynamics_in_existing_transaction;
use crate::aeroespacial::gravity::resolve_vehicle_gravitational_acceleration;
use crate::aeroespacial::propagation::{
    InitialAerodynamics, advance_vehicle_physical_state_with_initial_aerodynamics,
};
use crate::aeroespacial::vehicle::resolve_vehicle_snapshot;
use crate::environment::load_environment_snapshot;
use crate::error::{RocketError, RocketResult};
use crate::orbital::{invalidate_future_trajectory_patches, propagate_coasting_vehicle};
use crate::power::battery::apply_power_delta;
use crate::power::budget::resolve_vehicle_power_budget;
use crate::power::consumption::resolve_component_consumption;
use crate::power::generation::resolve_component_generation;
use crate::thermal::heat_shield_response::resolve_heat_shield_response;
use crate::thermal::network_assembly::assemble_vehicle_thermal_network;
use crate::thermal::network_tick::advance_vehicle_thermal_network_in_existing_transaction;
use astronomicon_app::climate::temperature::ClimateBodyInputs;
use astronomicon_app::ephemeris::resolve_planet_orientation;
use astronomicon_app::shape::effective_polar_radius_for_planet;
use astronomicon_core::math::rotation::angular_velocity_from_rotation_period;
use astronomicon_core::units::constants::STANDARD_GRAVITY;
use astronomicon_core::units::{AngularVelocityVector, Duration, Length, Luminosity, Vector3};
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::VehicleControlInput;
use rocketcon_core::math::collision::resolve_surface_contact;
use rocketcon_db::repositories::operational_state_repository;
use rocketcon_db::repositories::vehicle as vehicle_repository;
use rocketcon_db::repositories::vehicle_physical_state as vehicle_physical_state_repository;
use std::collections::HashMap;
use uuid::Uuid;

mod activation;
mod profile;
mod report;
mod transaction;

use activation::is_propulsion_or_control_active;
pub use profile::{TickProfile, TickStageTiming};
pub use report::VehicleTickReport;
pub use transaction::{
    advance_vehicle_simulation, advance_vehicle_simulation_in_session,
    advance_vehicle_simulation_profiled, advance_vehicle_simulation_profiled_in_session,
    advance_vehicle_simulation_profiled_with_transaction,
    advance_vehicle_simulation_with_transaction,
};

fn mark(profile: &mut Option<&mut TickProfile>, name: &'static str) {
    if let Some(profile) = profile.as_deref_mut() {
        profile.mark(name);
    }
}

pub(super) async fn advance_vehicle_simulation_inner(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    dt: Duration,
    universe_epoch: Duration,
    control_input: &VehicleControlInput,
    mut profile: Option<&mut TickProfile>,
) -> RocketResult<VehicleTickReport> {
    let current_physical_state =
        vehicle_physical_state_repository::get_by_vehicle_id(pool, &vehicle_id)
            .await?
            .ok_or_else(|| {
                RocketError::Generic(format!(
                    "physical state for vehicle '{}' not found",
                    vehicle_id
                ))
            })?;

    let current_at_epoch = current_physical_state.captured_at_epoch();
    let reference_body_id = current_physical_state.reference_body_id();
    let new_at_epoch = current_at_epoch + dt;

    let environment =
        load_environment_snapshot(pool, reference_body_id, universe_epoch, current_at_epoch)
            .await?;
    let climate_body = if environment.atmosphere.is_some() {
        Some(ClimateBodyInputs::load_in_existing_transaction(pool, reference_body_id).await?)
    } else {
        None
    };

    let vehicle_snapshot =
        resolve_vehicle_snapshot(pool, vehicle_id, universe_epoch, current_at_epoch).await?;
    mark(&mut profile, "load_state_environment_snapshot");

    let power_budget = resolve_vehicle_power_budget(
        pool,
        &vehicle_snapshot,
        &environment,
        current_physical_state.position(),
        universe_epoch,
        current_at_epoch,
    )
    .await?;

    let _dumped_power = apply_power_delta(
        pool,
        vehicle_id,
        power_budget.net_power,
        dt,
        universe_epoch,
        new_at_epoch,
    )
    .await?;
    mark(&mut profile, "power_and_battery");

    let components = vehicle_repository::list_components_for_vehicle(pool, &vehicle_id).await?;

    let mut component_waste_heats: HashMap<Uuid, Luminosity> =
        HashMap::with_capacity(components.len());
    for (entry, record) in &components {
        if !vehicle_snapshot.is_stage_active(entry.stage_index()) {
            continue;
        }

        let gen_contribution = resolve_component_generation(
            pool,
            entry,
            record,
            &environment,
            current_physical_state.position(),
            current_physical_state.orientation(),
            universe_epoch,
            current_at_epoch,
        )
        .await?;

        let op_state =
            operational_state_repository::get_by_vehicle_component_id(pool, &entry.id()).await?;
        let con_contribution = resolve_component_consumption(entry, record, op_state);

        let total_waste = gen_contribution.waste_heat + con_contribution.waste_heat;
        component_waste_heats.insert(entry.id(), total_waste);
    }
    mark(&mut profile, "component_heat_inputs");

    let is_propelling =
        is_propulsion_or_control_active(&vehicle_snapshot, &components, control_input);

    let aero_diag_current = resolve_vehicle_aerodynamics_in_existing_transaction(
        pool,
        &current_physical_state,
        reference_body_id,
        environment.planet_position.raw(),
        &components,
        vehicle_snapshot.active_stages(),
        universe_epoch,
        current_at_epoch,
        climate_body.as_ref(),
    )
    .await?;

    let has_drag = aero_diag_current
        .as_ref()
        .map_or(false, |a| a.drag_force.magnitude().value() > 1e-4);

    let planet = &environment.planet;
    let eq_radius = planet
        .equatorial_radius()
        .unwrap_or_else(|| Length::new(6371e3));
    let pol_radius = effective_polar_radius_for_planet(planet);

    let planet_orientation_current =
        resolve_planet_orientation(pool, planet.id(), universe_epoch, current_at_epoch).await?;

    let rot_period = planet
        .rotation_period()
        .unwrap_or_else(|| Duration::new(86400.0));
    let omega_mag = angular_velocity_from_rotation_period(rot_period);
    let spin_axis_current = planet_orientation_current.rotate_vector(Vector3::new(0.0, 0.0, 1.0));
    let planet_omega_current =
        AngularVelocityVector::from_raw(spin_axis_current * omega_mag.value());

    let surface_contact_current = resolve_surface_contact(
        eq_radius,
        pol_radius,
        environment.planet_position,
        planet_orientation_current,
        planet_omega_current,
        current_physical_state.position(),
        current_physical_state.velocity(),
    );

    let has_contact = surface_contact_current.has_contact();
    mark(&mut profile, "initial_aero_and_contact");

    let mut new_physical_state = if is_propelling || has_drag || has_contact {
        let initial_aerodynamics = InitialAerodynamics::new(
            current_physical_state,
            environment.planet_position.raw(),
            &components,
            vehicle_snapshot.active_stages(),
            universe_epoch,
            aero_diag_current,
        );
        let state = advance_vehicle_physical_state_with_initial_aerodynamics(
            pool,
            vehicle_id,
            dt,
            universe_epoch,
            control_input,
            initial_aerodynamics,
        )
        .await?;
        invalidate_future_trajectory_patches(pool, vehicle_id, universe_epoch + current_at_epoch)
            .await?;
        state
    } else {
        propagate_coasting_vehicle(pool, vehicle_id, dt, universe_epoch, current_at_epoch).await?
    };
    mark(&mut profile, "dynamics_or_coast");

    let planet_orientation_new =
        resolve_planet_orientation(pool, planet.id(), universe_epoch, new_at_epoch).await?;

    let spin_axis_new = planet_orientation_new.rotate_vector(Vector3::new(0.0, 0.0, 1.0));
    let planet_angular_velocity_new =
        AngularVelocityVector::from_raw(spin_axis_new * omega_mag.value());

    let total_epoch_new = universe_epoch + new_at_epoch;
    let positions_new = astronomicon_app::ephemeris::resolve_system_positions(
        pool,
        environment.system_id,
        total_epoch_new,
    )
    .await?;
    let planet_position_new = positions_new
        .get(&planet.id())
        .copied()
        .unwrap_or(environment.planet_position);

    let surface_contact = resolve_surface_contact(
        eq_radius,
        pol_radius,
        planet_position_new,
        planet_orientation_new,
        planet_angular_velocity_new,
        new_physical_state.position(),
        new_physical_state.velocity(),
    );

    let aero_diag = resolve_vehicle_aerodynamics_in_existing_transaction(
        pool,
        &new_physical_state,
        new_physical_state.reference_body_id(),
        planet_position_new.raw(),
        &components,
        vehicle_snapshot.active_stages(),
        universe_epoch,
        new_at_epoch,
        climate_body.as_ref(),
    )
    .await?;

    let raw_stag_flux = aero_diag
        .as_ref()
        .map(|a| a.stagnation_heat_flux)
        .unwrap_or_else(|| astronomicon_core::units::HeatFlux::new(0.0));

    if raw_stag_flux.value() > 0.0 {
        let _shield_response = resolve_heat_shield_response(
            pool,
            &components,
            vehicle_snapshot.active_stages(),
            raw_stag_flux,
            dt,
            universe_epoch,
            current_at_epoch,
        )
        .await?;
    }
    mark(&mut profile, "final_ephemeris_aero_shield");

    let (air_density, relative_airspeed, mach_number) = match aero_diag {
        Some(ref d) => (
            Some(d.air_density),
            Some(d.relative_airspeed),
            Some(d.mach_number),
        ),
        None => (None, None, None),
    };

    let mut thermal_network = assemble_vehicle_thermal_network(
        pool,
        vehicle_id,
        &components,
        vehicle_snapshot.active_stages(),
        &component_waste_heats,
        air_density,
        relative_airspeed,
        mach_number,
        None,
    )
    .await?;

    let thermal_tick_report = advance_vehicle_thermal_network_in_existing_transaction(
        pool,
        vehicle_id,
        &mut thermal_network,
        &components,
        vehicle_snapshot.active_stages(),
        &environment,
        new_physical_state.position(),
        new_physical_state.orientation(),
        None,
        dt,
        universe_epoch,
        current_at_epoch,
    )
    .await?;
    mark(&mut profile, "thermal_network");

    let grav_acc = resolve_vehicle_gravitational_acceleration(
        pool,
        &environment,
        &new_physical_state,
        universe_epoch,
        new_at_epoch,
    )
    .await?;

    let (max_q, max_q_epoch) = match (new_physical_state.max_dynamic_pressure(), aero_diag) {
        (Some(prev_q), Some(diag)) => {
            if diag.dynamic_pressure.value() > prev_q.value() {
                (Some(diag.dynamic_pressure), Some(total_epoch_new))
            } else {
                (
                    Some(prev_q),
                    new_physical_state.max_dynamic_pressure_epoch(),
                )
            }
        }
        (None, Some(diag)) => (Some(diag.dynamic_pressure), Some(total_epoch_new)),
        (Some(prev_q), None) => (
            Some(prev_q),
            new_physical_state.max_dynamic_pressure_epoch(),
        ),
        (None, None) => (None, None),
    };

    if max_q != new_physical_state.max_dynamic_pressure() {
        new_physical_state = new_physical_state.with_max_dynamic_pressure(max_q, max_q_epoch);
        vehicle_physical_state_repository::upsert(pool, &new_physical_state).await?;
    }

    let dt_val = dt.value();
    let net_linear_acc = if dt_val > 0.0 && dt_val.is_finite() {
        (new_physical_state.velocity().raw() - current_physical_state.velocity().raw()) / dt_val
    } else {
        Vector3::zero()
    };

    let proper_acc_world = if surface_contact.has_contact() {
        let n = surface_contact.surface_normal_world();
        let g_proj = grav_acc.raw().dot(&n);
        if g_proj < 0.0 {
            -n * g_proj
        } else {
            -grav_acc.raw()
        }
    } else {
        net_linear_acc - grav_acc.raw()
    };

    let proper_acc_body = new_physical_state
        .orientation()
        .inverse()
        .rotate_vector(proper_acc_world);

    let axial_g_load = proper_acc_body.2 / STANDARD_GRAVITY;
    let lateral_g_load =
        (proper_acc_body.0 * proper_acc_body.0 + proper_acc_body.1 * proper_acc_body.1).sqrt()
            / STANDARD_GRAVITY;
    let total_g_load = proper_acc_body.magnitude() / STANDARD_GRAVITY;

    let mut final_thermal_budget = thermal_tick_report.budget;
    if let Some(ref d) = aero_diag {
        final_thermal_budget.stagnation_heat_flux = d.stagnation_heat_flux;
    }
    mark(&mut profile, "gravity_maxq_and_report");

    Ok(VehicleTickReport::new(
        new_physical_state,
        aero_diag,
        grav_acc,
        surface_contact,
        power_budget,
        final_thermal_budget,
        axial_g_load,
        lateral_g_load,
        total_g_load,
        [planet_position_new.raw().0, planet_position_new.raw().1, planet_position_new.raw().2],
        eq_radius.value(),
    ))
}
