use crate::constants::{
    ATMOSPHERIC_PASS_DEFAULT_DT_S, ATMOSPHERIC_PASS_DEFAULT_MAX_DURATION_S,
    CORRIDOR_BISECTION_ITERATIONS, CORRIDOR_SEARCH_STEPS, CORRIDOR_SHALLOW_GAMMA_INITIAL_RAD,
    CORRIDOR_STEEP_GAMMA_INITIAL_RAD,
};
use crate::error::RocketDomainResult;
use crate::math::aerothermodynamics::kinematics::{
    state_from_entry_parameters, vacuum_periapsis_from_entry_angle,
};
use crate::math::aerothermodynamics::pass_simulation::simulate_atmospheric_pass;
use crate::math::aerothermodynamics::types::{
    AerocaptureOutcome, AerocaptureTrajectoryResult, AerocaptureVehicleProperties,
    AtmosphericModelParameters, EntryCorridor,
};
use astronomicon_core::units::{Angle, Duration, Length, Speed};

pub fn analyze_entry_corridor(
    entry_speed: Speed,
    inclination: Angle,
    atm_params: &AtmosphericModelParameters,
    vehicle_props: &AerocaptureVehicleProperties,
    target_apoapsis_max: Length,
) -> RocketDomainResult<EntryCorridor> {
    let r_entry = atm_params.entry_interface_radius;
    let gamma_shallow = CORRIDOR_SHALLOW_GAMMA_INITIAL_RAD;
    let gamma_steep = CORRIDOR_STEEP_GAMMA_INITIAL_RAD;

    let eval_gamma = |gamma_val: f64| -> RocketDomainResult<AerocaptureTrajectoryResult> {
        let (pos, vel) = state_from_entry_parameters(
            r_entry,
            entry_speed,
            Angle::new(gamma_val),
            inclination,
            Angle::new(0.0),
        );
        simulate_atmospheric_pass(
            pos,
            vel,
            Duration::new(0.0),
            atm_params,
            vehicle_props,
            Duration::new(ATMOSPHERIC_PASS_DEFAULT_MAX_DURATION_S),
            Duration::new(ATMOSPHERIC_PASS_DEFAULT_DT_S),
        )
    };

    let mut found_shallow: Option<f64> = None;
    let steps = CORRIDOR_SEARCH_STEPS;
    let d_gamma = (gamma_steep - gamma_shallow) / (steps as f64);

    for i in 0..=steps {
        let g = gamma_shallow + (i as f64) * d_gamma;
        let res = eval_gamma(g)?;
        if let AerocaptureOutcome::Captured {
            post_pass_elements, ..
        } = &res.outcome
        {
            if let Some(r_apo) = post_pass_elements.apoapsis_distance() {
                if r_apo.value() <= target_apoapsis_max.value() {
                    let mut g_low = g - d_gamma;
                    let mut g_high = g;
                    for _ in 0..CORRIDOR_BISECTION_ITERATIONS {
                        let g_mid = 0.5 * (g_low + g_high);
                        let sub_res = eval_gamma(g_mid)?;
                        if let AerocaptureOutcome::Captured {
                            post_pass_elements: sub_el,
                            ..
                        } = &sub_res.outcome
                        {
                            if sub_el
                                .apoapsis_distance()
                                .map_or(false, |a| a.value() <= target_apoapsis_max.value())
                            {
                                g_high = g_mid;
                            } else {
                                g_low = g_mid;
                            }
                        } else {
                            g_low = g_mid;
                        }
                    }
                    found_shallow = Some(g_high);
                    break;
                }
            }
        }
    }

    let mut found_steep: Option<f64> = None;
    for i in (0..=steps).rev() {
        let g = gamma_shallow + (i as f64) * d_gamma;
        let res = eval_gamma(g)?;
        let safe = match &res.outcome {
            AerocaptureOutcome::Captured { .. } => {
                res.peak_stagnation_heat_flux.value()
                    <= vehicle_props.max_allowable_heat_flux.value()
                    && res.peak_g_load <= vehicle_props.max_allowable_g_load
                    && res.periapsis_altitude.value() > 0.0
            }
            _ => false,
        };

        if safe {
            let mut g_low = g + d_gamma;
            let mut g_high = g;
            for _ in 0..CORRIDOR_BISECTION_ITERATIONS {
                let g_mid = 0.5 * (g_low + g_high);
                let sub_res = eval_gamma(g_mid)?;
                let sub_safe = match &sub_res.outcome {
                    AerocaptureOutcome::Captured { .. } => {
                        sub_res.peak_stagnation_heat_flux.value()
                            <= vehicle_props.max_allowable_heat_flux.value()
                            && sub_res.peak_g_load <= vehicle_props.max_allowable_g_load
                            && sub_res.periapsis_altitude.value() > 0.0
                    }
                    _ => false,
                };
                if sub_safe {
                    g_high = g_mid;
                } else {
                    g_low = g_mid;
                }
            }
            found_steep = Some(g_high);
            break;
        }
    }

    match (found_shallow, found_steep) {
        (Some(g_sh), Some(g_st)) if g_st <= g_sh => {
            let width = (g_sh - g_st).abs();
            let rp_sh = vacuum_periapsis_from_entry_angle(
                r_entry,
                entry_speed,
                Angle::new(g_sh),
                atm_params.gravitational_parameter,
            );
            let rp_st = vacuum_periapsis_from_entry_angle(
                r_entry,
                entry_speed,
                Angle::new(g_st),
                atm_params.gravitational_parameter,
            );
            let p_rad = atm_params.planet_radius.value();

            Ok(EntryCorridor {
                overshoot_flight_path_angle: Angle::new(g_sh),
                undershoot_flight_path_angle: Angle::new(g_st),
                corridor_width: Angle::new(width),
                overshoot_periapsis_altitude: Length::new(rp_sh.value() - p_rad),
                undershoot_periapsis_altitude: Length::new(rp_st.value() - p_rad),
                is_viable: width > 1e-4,
            })
        }
        _ => Ok(EntryCorridor {
            overshoot_flight_path_angle: Angle::new(0.0),
            undershoot_flight_path_angle: Angle::new(0.0),
            corridor_width: Angle::new(0.0),
            overshoot_periapsis_altitude: Length::new(0.0),
            undershoot_periapsis_altitude: Length::new(0.0),
            is_viable: false,
        }),
    }
}