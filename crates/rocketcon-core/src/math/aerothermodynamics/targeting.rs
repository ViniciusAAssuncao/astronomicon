use crate::constants::{
    ATMOSPHERIC_PASS_DEFAULT_DT_S, ATMOSPHERIC_PASS_DEFAULT_MAX_DURATION_S,
    ATMOSPHERIC_PASS_FINE_DT_S, DEFAULT_FALLBACK_ENTRY_ANGLE_RAD,
    TARGET_APOAPSIS_BISECTION_ITERATIONS, TARGET_APOAPSIS_TOLERANCE_M,
    TARGET_CORRIDOR_APOAPSIS_MULTIPLIER, TARGET_GAMMA_TOLERANCE_RAD,
};
use crate::error::RocketDomainResult;
use crate::math::aerothermodynamics::corridor::analyze_entry_corridor;
use crate::math::aerothermodynamics::kinematics::{
    state_from_entry_parameters, vacuum_periapsis_from_entry_angle,
};
use crate::math::aerothermodynamics::pass_simulation::simulate_atmospheric_pass;
use crate::math::aerothermodynamics::types::{
    AerocaptureOutcome, AerocapturePlan, AerocaptureTrajectoryResult,
    AerocaptureVehicleProperties, AtmosphericModelParameters,
};
use astronomicon_core::units::{Angle, Duration, Length, Speed};

pub fn target_aerocapture_entry_angle(
    entry_speed: Speed,
    inclination: Angle,
    target_apoapsis: Length,
    atm_params: &AtmosphericModelParameters,
    vehicle_props: &AerocaptureVehicleProperties,
) -> RocketDomainResult<AerocapturePlan> {
    let r_entry = atm_params.entry_interface_radius;
    let r_target = target_apoapsis.value();

    let corridor = analyze_entry_corridor(
        entry_speed,
        inclination,
        atm_params,
        vehicle_props,
        Length::new(r_target * TARGET_CORRIDOR_APOAPSIS_MULTIPLIER),
    )?;

    if !corridor.is_viable {
        let (pos, vel) = state_from_entry_parameters(
            r_entry,
            entry_speed,
            Angle::new(DEFAULT_FALLBACK_ENTRY_ANGLE_RAD),
            inclination,
            Angle::new(0.0),
        );
        let traj = simulate_atmospheric_pass(
            pos,
            vel,
            Duration::new(0.0),
            atm_params,
            vehicle_props,
            Duration::new(ATMOSPHERIC_PASS_DEFAULT_MAX_DURATION_S),
            Duration::new(ATMOSPHERIC_PASS_DEFAULT_DT_S),
        )?;
        return Ok(AerocapturePlan {
            target_entry_flight_path_angle: Angle::new(DEFAULT_FALLBACK_ENTRY_ANGLE_RAD),
            target_vacuum_periapsis_altitude: Length::new(0.0),
            target_vacuum_periapsis_radius: Length::new(0.0),
            predicted_post_pass_apoapsis: Length::new(0.0),
            predicted_post_pass_periapsis: Length::new(0.0),
            peak_stagnation_heat_flux: traj.peak_stagnation_heat_flux,
            peak_dynamic_pressure: traj.peak_dynamic_pressure,
            peak_g_load: traj.peak_g_load,
            integrated_heat_load_j_per_m2: traj.integrated_heat_load_j_per_m2,
            total_delta_v_absorbed: traj.total_delta_v_absorbed,
            atmospheric_pass_duration: traj.exit_epoch.unwrap_or_else(|| Duration::new(0.0)),
            corridor,
            trajectory: traj,
            is_feasible: false,
        });
    }

    let mut g_low = corridor.undershoot_flight_path_angle.value();
    let mut g_high = corridor.overshoot_flight_path_angle.value();

    let eval = |g: f64| -> RocketDomainResult<(f64, AerocaptureTrajectoryResult)> {
        let (pos, vel) = state_from_entry_parameters(
            r_entry,
            entry_speed,
            Angle::new(g),
            inclination,
            Angle::new(0.0),
        );
        let res = simulate_atmospheric_pass(
            pos,
            vel,
            Duration::new(0.0),
            atm_params,
            vehicle_props,
            Duration::new(ATMOSPHERIC_PASS_DEFAULT_MAX_DURATION_S),
            Duration::new(ATMOSPHERIC_PASS_FINE_DT_S),
        )?;
        let apo_val = match &res.outcome {
            AerocaptureOutcome::Captured {
                post_pass_elements, ..
            } => post_pass_elements
                .apoapsis_distance()
                .map(|a| a.value())
                .unwrap_or(f64::INFINITY),
            _ => f64::INFINITY,
        };
        Ok((apo_val, res))
    };

    let mut best_g = 0.5 * (g_low + g_high);
    let mut best_traj = eval(best_g)?.1;

    for _ in 0..TARGET_APOAPSIS_BISECTION_ITERATIONS {
        let g_mid = 0.5 * (g_low + g_high);
        let (apo_mid, traj_mid) = eval(g_mid)?;

        best_g = g_mid;
        best_traj = traj_mid;

        if (apo_mid - r_target).abs() < TARGET_APOAPSIS_TOLERANCE_M
            || (g_high - g_low).abs() < TARGET_GAMMA_TOLERANCE_RAD
        {
            break;
        }

        if apo_mid > r_target {
            g_high = g_mid;
        } else {
            g_low = g_mid;
        }
    }

    let is_feasible = match &best_traj.outcome {
        AerocaptureOutcome::Captured { .. } => {
            best_traj.peak_stagnation_heat_flux.value()
                <= vehicle_props.max_allowable_heat_flux.value()
                && best_traj.peak_g_load <= vehicle_props.max_allowable_g_load
                && best_traj.periapsis_altitude.value() > 0.0
        }
        _ => false,
    };

    let rp_vac = vacuum_periapsis_from_entry_angle(
        r_entry,
        entry_speed,
        Angle::new(best_g),
        atm_params.gravitational_parameter,
    );
    let p_rad = atm_params.planet_radius.value();

    let post_apo = best_traj.post_pass_apoapsis.unwrap_or(Length::new(r_target));
    let post_peri = best_traj.post_pass_periapsis.unwrap_or(Length::new(p_rad));

    let pass_dur = match best_traj.exit_epoch {
        Some(exit) => exit - best_traj.entry_epoch,
        None => Duration::new(0.0),
    };

    Ok(AerocapturePlan {
        target_entry_flight_path_angle: Angle::new(best_g),
        target_vacuum_periapsis_altitude: Length::new(rp_vac.value() - p_rad),
        target_vacuum_periapsis_radius: rp_vac,
        predicted_post_pass_apoapsis: post_apo,
        predicted_post_pass_periapsis: post_peri,
        peak_stagnation_heat_flux: best_traj.peak_stagnation_heat_flux,
        peak_dynamic_pressure: best_traj.peak_dynamic_pressure,
        peak_g_load: best_traj.peak_g_load,
        integrated_heat_load_j_per_m2: best_traj.integrated_heat_load_j_per_m2,
        total_delta_v_absorbed: best_traj.total_delta_v_absorbed,
        atmospheric_pass_duration: pass_dur,
        corridor,
        trajectory: best_traj,
        is_feasible,
    })
}

pub fn plan_aerocapture(
    entry_speed: Speed,
    inclination: Angle,
    target_apoapsis: Length,
    atm_params: &AtmosphericModelParameters,
    vehicle_props: &AerocaptureVehicleProperties,
) -> RocketDomainResult<AerocapturePlan> {
    target_aerocapture_entry_angle(
        entry_speed,
        inclination,
        target_apoapsis,
        atm_params,
        vehicle_props,
    )
}