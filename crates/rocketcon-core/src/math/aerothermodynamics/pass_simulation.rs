use crate::constants::{
    ATMOSPHERIC_ENTRY_ALTITUDE_BUFFER_M,
    ATMOSPHERIC_PASS_EXO_DT_MULTIPLIER,
    ATMOSPHERIC_PASS_HIGH_DENSITY_THRESHOLD,
    ATMOSPHERIC_PASS_MAX_DT_S,
    ATMOSPHERIC_PASS_MIN_DT_S,
    ATMOSPHERIC_PASS_MIN_DURATION_S,
};
use crate::domain::trajectory_patch::LowThrustPatchData;
use crate::error::{ RocketDomainError, RocketDomainResult };
use crate::math::aerothermodynamics::dynamics::atmospheric_derivatives;
use crate::math::aerothermodynamics::types::{
    AerocaptureOutcome,
    AerocaptureTrajectoryResult,
    AerocaptureVehicleProperties,
    AtmosphericModelParameters,
    AtmosphericPassState,
};
use crate::math::orbital::conversions::cartesian_to_osculating_elements;
use crate::math::orbital::low_thrust::fit_chebyshev_coefficients;
use crate::math::orbital::types::{ OrbitType, OsculatingElements };
use astronomicon_core::units::{
    Angle,
    Density,
    Duration,
    Force,
    HeatFlux,
    Length,
    Mass,
    Position,
    Pressure,
    Speed,
    Vector3,
    VelocityVector,
};

pub fn simulate_atmospheric_pass(
    entry_position: Position,
    entry_velocity: VelocityVector,
    entry_epoch: Duration,
    atm_params: &AtmosphericModelParameters,
    vehicle_props: &AerocaptureVehicleProperties,
    max_duration: Duration,
    base_time_step: Duration
) -> RocketDomainResult<AerocaptureTrajectoryResult> {
    let planet_r = atm_params.planet_radius.value();
    let r_entry = atm_params.entry_interface_radius.value();
    let max_t = max_duration.value().max(ATMOSPHERIC_PASS_MIN_DURATION_S);
    let dt_base = base_time_step
        .value()
        .clamp(ATMOSPHERIC_PASS_MIN_DT_S * 5.0, ATMOSPHERIC_PASS_MAX_DT_S);

    let mut pos = entry_position.raw();
    let mut vel = entry_velocity.raw();
    let mut t = entry_epoch.value();
    let t_start = t;

    let mut states = Vec::with_capacity(500);

    let mut min_alt = f64::INFINITY;
    let mut max_q = 0.0;
    let mut max_flux = 0.0;
    let mut max_g = 0.0;
    let mut heat_load = 0.0;
    let v_entry_mag = vel.magnitude();

    let mut has_descended = false;
    let mut outcome: Option<AerocaptureOutcome> = None;
    let mut exit_epoch: Option<Duration> = None;

    while t - t_start <= max_t {
        let r = pos.magnitude();
        let alt = r - planet_r;
        let v_mag = vel.magnitude();

        if alt < min_alt {
            min_alt = alt;
        }

        let u_r = pos / r.max(1.0);
        let u_v = vel / v_mag.max(1e-6);
        let gamma = u_r.dot(&u_v).clamp(-1.0, 1.0).asin();

        let (_, _, q_curr, flux_curr, g_curr, rho_curr) = atmospheric_derivatives(
            pos,
            vel,
            atm_params,
            vehicle_props
        );

        if q_curr > max_q {
            max_q = q_curr;
        }
        if flux_curr > max_flux {
            max_flux = flux_curr;
        }
        if g_curr > max_g {
            max_g = g_curr;
        }

        states.push(AtmosphericPassState {
            time: Duration::new(t),
            position: Position::from_raw(pos),
            velocity: VelocityVector::from_raw(vel),
            altitude: Length::new(alt),
            speed: Speed::new(v_mag),
            flight_path_angle: Angle::new(gamma),
            density: Density::new(rho_curr),
            dynamic_pressure: Pressure::new(q_curr),
            stagnation_heat_flux: HeatFlux::new(flux_curr),
            g_load: g_curr,
        });

        if alt <= 0.0 {
            outcome = Some(AerocaptureOutcome::SurfaceImpact {
                impact_speed: Speed::new(v_mag),
                impact_time: Duration::new(t - t_start),
            });
            break;
        }

        if flux_curr > vehicle_props.max_allowable_heat_flux.value() {
            outcome = Some(AerocaptureOutcome::ExceededThermalLimits {
                peak_heat_flux: HeatFlux::new(flux_curr),
                limit: vehicle_props.max_allowable_heat_flux,
            });
            break;
        }

        if g_curr > vehicle_props.max_allowable_g_load {
            outcome = Some(AerocaptureOutcome::ExceededStructuralLimits {
                peak_g_load: g_curr,
                limit: vehicle_props.max_allowable_g_load,
            });
            break;
        }

        if r < r_entry - ATMOSPHERIC_ENTRY_ALTITUDE_BUFFER_M {
            has_descended = true;
        }

        if has_descended && r >= r_entry && vel.dot(&pos) > 0.0 {
            let post_elements = cartesian_to_osculating_elements(
                Position::from_raw(pos),
                VelocityVector::from_raw(vel),
                atm_params.gravitational_parameter
            )?;

            exit_epoch = Some(Duration::new(t));
            outcome = if post_elements.is_bound() {
                Some(AerocaptureOutcome::Captured {
                    post_pass_elements: post_elements,
                    exit_epoch: Duration::new(t),
                })
            } else {
                Some(AerocaptureOutcome::Escaped {
                    exit_elements: post_elements,
                    exit_epoch: Duration::new(t),
                })
            };
            break;
        }

        let dt = if rho_curr > ATMOSPHERIC_PASS_HIGH_DENSITY_THRESHOLD {
            let scale_h = atm_params.scale_height.value();
            let v_vert = v_mag * gamma.sin().abs();
            let dt_scale = scale_h / (v_vert + 100.0);
            let dt_g = 20.0 / (g_curr + 1.0);
            dt_base.min(dt_scale).min(dt_g).clamp(ATMOSPHERIC_PASS_MIN_DT_S, dt_base)
        } else {
            dt_base * ATMOSPHERIC_PASS_EXO_DT_MULTIPLIER
        };

        let half_dt = 0.5 * dt;

        let (v1, a1, _, fl1, _, _) = atmospheric_derivatives(pos, vel, atm_params, vehicle_props);

        let p2 = pos + v1 * half_dt;
        let v2_step = vel + a1 * half_dt;
        let (v2, a2, _, fl2, _, _) = atmospheric_derivatives(
            p2,
            v2_step,
            atm_params,
            vehicle_props
        );

        let p3 = pos + v2 * half_dt;
        let v3_step = vel + a2 * half_dt;
        let (v3, a3, _, fl3, _, _) = atmospheric_derivatives(
            p3,
            v3_step,
            atm_params,
            vehicle_props
        );

        let p4 = pos + v3 * dt;
        let v4_step = vel + a3 * dt;
        let (v4, a4, _, fl4, _, _) = atmospheric_derivatives(
            p4,
            v4_step,
            atm_params,
            vehicle_props
        );

        let sixth_dt = dt / 6.0;
        pos = pos + (v1 + v2 * 2.0 + v3 * 2.0 + v4) * sixth_dt;
        vel = vel + (a1 + a2 * 2.0 + a3 * 2.0 + a4) * sixth_dt;

        let avg_flux = (fl1 + 2.0 * fl2 + 2.0 * fl3 + fl4) / 6.0;
        heat_load += avg_flux * dt;
        t += dt;
    }

    let final_outcome = outcome.unwrap_or_else(|| {
        let post_elements = cartesian_to_osculating_elements(
            Position::from_raw(pos),
            VelocityVector::from_raw(vel),
            atm_params.gravitational_parameter
        ).unwrap_or_else(|_| {
            OsculatingElements::new(
                Length::new(0.0),
                0.0,
                Angle::new(0.0),
                Angle::new(0.0),
                Angle::new(0.0),
                Angle::new(0.0),
                Length::new(0.0),
                None,
                0.0,
                Vector3::zero(),
                OrbitType::Circular
            )
        });

        if post_elements.is_bound() {
            AerocaptureOutcome::Captured {
                post_pass_elements: post_elements,
                exit_epoch: Duration::new(t),
            }
        } else {
            AerocaptureOutcome::Escaped {
                exit_elements: post_elements,
                exit_epoch: Duration::new(t),
            }
        }
    });

    let (post_apo, post_peri) = match &final_outcome {
        AerocaptureOutcome::Captured { post_pass_elements, .. } =>
            (post_pass_elements.apoapsis_distance(), Some(post_pass_elements.periapsis_distance())),
        AerocaptureOutcome::Escaped { exit_elements, .. } =>
            (exit_elements.apoapsis_distance(), Some(exit_elements.periapsis_distance())),
        _ => (None, None),
    };

    let v_exit_mag = vel.magnitude();
    let dv_absorbed = (v_entry_mag - v_exit_mag).max(0.0);

    Ok(AerocaptureTrajectoryResult {
        states,
        outcome: final_outcome,
        entry_epoch,
        exit_epoch,
        periapsis_altitude: Length::new(min_alt),
        peak_dynamic_pressure: Pressure::new(max_q),
        peak_stagnation_heat_flux: HeatFlux::new(max_flux),
        integrated_heat_load_j_per_m2: heat_load,
        peak_g_load: max_g,
        post_pass_apoapsis: post_apo,
        post_pass_periapsis: post_peri,
        total_delta_v_absorbed: Speed::new(dv_absorbed),
    })
}

pub fn trajectory_to_chebyshev_patch(
    trajectory: &AerocaptureTrajectoryResult,
    degree: usize
) -> RocketDomainResult<LowThrustPatchData> {
    let n = trajectory.states.len();
    if n < degree + 1 {
        return Err(RocketDomainError::InvalidInvariant {
            field: "states".to_string(),
            reason: "insufficient trajectory states to construct polynomial patch".to_string(),
        });
    }

    let total_time = match trajectory.exit_epoch {
        Some(exit) => (exit - trajectory.entry_epoch).value(),
        None => {
            let last_t = trajectory.states
                .last()
                .map(|s| s.time.value())
                .unwrap_or(0.0);
            (last_t - trajectory.entry_epoch.value()).max(0.1)
        }
    };

    let mut tau_samples = Vec::with_capacity(n);
    let mut xs = Vec::with_capacity(n);
    let mut ys = Vec::with_capacity(n);
    let mut zs = Vec::with_capacity(n);
    let mut vxs = Vec::with_capacity(n);
    let mut vys = Vec::with_capacity(n);
    let mut vzs = Vec::with_capacity(n);
    let mut masses = Vec::with_capacity(n);

    let t0 = trajectory.entry_epoch.value();
    for state in &trajectory.states {
        let t = state.time.value();
        let tau = if total_time > 0.0 { (2.0 * (t - t0)) / total_time - 1.0 } else { 0.0 };
        tau_samples.push(tau.clamp(-1.0, 1.0));

        let p = state.position.raw();
        let v = state.velocity.raw();
        xs.push(p.0);
        ys.push(p.1);
        zs.push(p.2);
        vxs.push(v.0);
        vys.push(v.1);
        vzs.push(v.2);
        masses.push(1.0);
    }

    let c_x = fit_chebyshev_coefficients(&tau_samples, &xs, degree)?;
    let c_y = fit_chebyshev_coefficients(&tau_samples, &ys, degree)?;
    let c_z = fit_chebyshev_coefficients(&tau_samples, &zs, degree)?;
    let c_vx = fit_chebyshev_coefficients(&tau_samples, &vxs, degree)?;
    let c_vy = fit_chebyshev_coefficients(&tau_samples, &vys, degree)?;
    let c_vz = fit_chebyshev_coefficients(&tau_samples, &vzs, degree)?;
    let c_mass = fit_chebyshev_coefficients(&tau_samples, &masses, degree)?;

    Ok(LowThrustPatchData {
        initial_mass: Mass::new(1.0),
        final_mass: Mass::new(1.0),
        thrust: Force::new(0.0),
        specific_impulse: Duration::new(1.0),
        total_delta_v: trajectory.total_delta_v_absorbed,
        chebyshev_x: c_x,
        chebyshev_y: c_y,
        chebyshev_z: c_z,
        chebyshev_vx: c_vx,
        chebyshev_vy: c_vy,
        chebyshev_vz: c_vz,
        chebyshev_mass: c_mass,
    })
}
