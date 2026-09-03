use crate::domain::{ConicPatchData, TrajectoryPatch, TrajectoryPatchKind};
use crate::error::{RocketDomainError, RocketDomainResult};
use crate::math::aerothermodynamics::aerocapture::{
    simulate_atmospheric_pass, AerocaptureOutcome, AerocaptureVehicleProperties,
    AtmosphericModelParameters,
};
use crate::math::orbital::hyperbolic::{
    hyperbolic_mean_anomaly_from_true_anomaly, hyperbolic_mean_motion,
};
use crate::math::orbital::parabolic::parabolic_time_since_periapsis;
use crate::math::orbital::sphere_of_influence::{
    transform_state_from_old_soi, transform_state_to_new_soi, CelestialBodySoi,
};
use crate::math::orbital::universal::propagate_universal_state_vectors;
use crate::math::orbital::{
    cartesian_to_osculating_elements, OrbitType, OsculatingElements,
};
use astronomicon_core::math::kepler::mean_motion;
use astronomicon_core::units::{
    Angle, Duration, GravitationalParameter, HeatFlux, Length, Mass, Position,
    VelocityVector,
};
use std::f64::consts::TAU;
use uuid::Uuid;

pub fn semi_latus_rectum(elements: &OsculatingElements) -> f64 {
    let e = elements.eccentricity();
    let a = elements.semi_major_axis().value();
    if elements.orbit_type() == OrbitType::Parabolic || (1.0 - e).abs() < 1e-6 {
        2.0 * elements.periapsis_distance().value()
    } else if e < 1.0 {
        a * (1.0 - e * e)
    } else {
        (-a) * (e * e - 1.0)
    }
}

pub fn true_anomaly_at_radius(
    elements: &OsculatingElements,
    radius: Length,
) -> Option<(Angle, Angle)> {
    let r = radius.value();
    let e = elements.eccentricity();
    let p = semi_latus_rectum(elements);

    if r <= 0.0 || p <= 0.0 || !r.is_finite() || !p.is_finite() {
        return None;
    }

    if e < 1e-8 {
        return None;
    }

    let cos_nu = (p / r - 1.0) / e;
    if cos_nu < -1.0 || cos_nu > 1.0 {
        return None;
    }

    let nu_pos = cos_nu.acos();
    Some((Angle::new(nu_pos), Angle::new(TAU - nu_pos)))
}

pub fn time_from_periapsis_to_true_anomaly(
    elements: &OsculatingElements,
    mu: GravitationalParameter,
    true_anomaly: Angle,
) -> RocketDomainResult<Duration> {
    let e = elements.eccentricity();
    let nu = true_anomaly.value();

    if e < 1.0 {
        let n = mean_motion(elements.semi_major_axis(), mu).value();
        if n <= 0.0 || !n.is_finite() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "mean_motion".to_string(),
                reason: "mean motion must be positive and finite".to_string(),
            });
        }
        let cos_e = (e + nu.cos()) / (1.0 + e * nu.cos());
        let sin_e = ((1.0 - e * e).sqrt() * nu.sin()) / (1.0 + e * nu.cos());
        let e_anom = sin_e.atan2(cos_e).rem_euclid(TAU);
        let m = (e_anom - e * e_anom.sin()).rem_euclid(TAU);
        Ok(Duration::new(m / n))
    } else if (1.0 - e).abs() < 1e-6 {
        Ok(parabolic_time_since_periapsis(
            elements.periapsis_distance(),
            mu,
            true_anomaly,
        ))
    } else {
        let n_h = hyperbolic_mean_motion(elements.semi_major_axis(), mu).value();
        if n_h <= 0.0 || !n_h.is_finite() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "hyperbolic_mean_motion".to_string(),
                reason: "hyperbolic mean motion must be positive and finite".to_string(),
            });
        }
        let m_h = hyperbolic_mean_anomaly_from_true_anomaly(true_anomaly, e)?;
        Ok(Duration::new(m_h / n_h))
    }
}

pub fn time_between_true_anomalies(
    elements: &OsculatingElements,
    mu: GravitationalParameter,
    nu_from: Angle,
    nu_to: Angle,
) -> RocketDomainResult<Duration> {
    let t_from = time_from_periapsis_to_true_anomaly(elements, mu, nu_from)?;
    let t_to = time_from_periapsis_to_true_anomaly(elements, mu, nu_to)?;

    if elements.eccentricity() < 1.0 {
        let period = TAU / mean_motion(elements.semi_major_axis(), mu).value();
        let mut dt = t_to.value() - t_from.value();
        if dt < 0.0 {
            dt += period;
        }
        Ok(Duration::new(dt))
    } else {
        let dt = t_to.value() - t_from.value();
        if dt < 0.0 {
            Err(RocketDomainError::InvalidInvariant {
                field: "true_anomaly_interval".to_string(),
                reason: "target true anomaly is in the past for escape trajectory".to_string(),
            })
        } else {
            Ok(Duration::new(dt))
        }
    }
}

pub fn find_soi_exit_event(
    elements: &OsculatingElements,
    mu: GravitationalParameter,
    soi_radius: Length,
) -> RocketDomainResult<Option<(Angle, Duration)>> {
    let r_soi = soi_radius.value();
    if r_soi <= 0.0 || !r_soi.is_finite() {
        return Ok(None);
    }

    if let Some(r_apo) = elements.apoapsis_distance() {
        if r_apo.value() < r_soi {
            return Ok(None);
        }
    }

    let Some((nu1, nu2)) = true_anomaly_at_radius(elements, soi_radius) else {
        return Ok(None);
    };

    let nu_curr = elements.true_anomaly().value().rem_euclid(TAU);
    let nu_exit = if elements.eccentricity() < 1.0 {
        if nu_curr <= nu1.value() {
            nu1
        } else if nu_curr <= nu2.value() {
            nu2
        } else {
            nu1
        }
    } else {
        nu1
    };

    let dt = time_between_true_anomalies(elements, mu, elements.true_anomaly(), nu_exit)?;
    Ok(Some((nu_exit, dt)))
}

pub fn find_body_encounter(
    initial_pos: Position,
    initial_vel: VelocityVector,
    primary_mu: GravitationalParameter,
    target_body: &CelestialBodySoi,
    max_search_duration: Duration,
    steps: usize,
) -> RocketDomainResult<Option<Duration>> {
    let dt_total = max_search_duration.value();
    if dt_total <= 0.0 || steps < 10 {
        return Ok(None);
    }

    let h = dt_total / (steps as f64);
    let r_target = target_body.soi_radius().value();
    let target_r_sq = r_target * r_target;

    let dist_sq_at = |t_sec: f64| -> RocketDomainResult<f64> {
        let (r_v, _) = propagate_universal_state_vectors(
            initial_pos,
            initial_vel,
            primary_mu,
            Duration::new(t_sec),
        )?;
        let diff = r_v.raw() - target_body.position().raw();
        Ok(diff.dot(&diff))
    };

    let mut prev_t = 0.0;
    let prev_f = dist_sq_at(0.0)? - target_r_sq;

    if prev_f <= 0.0 {
        return Ok(Some(Duration::new(0.0)));
    }

    for i in 1..=steps {
        let curr_t = (i as f64) * h;
        let curr_f = dist_sq_at(curr_t)? - target_r_sq;

        if curr_f <= 0.0 {
            let mut t_low = prev_t;
            let mut t_high = curr_t;
            for _ in 0..40 {
                let t_mid = 0.5 * (t_low + t_high);
                let f_mid = dist_sq_at(t_mid)? - target_r_sq;
                if f_mid <= 0.0 {
                    t_high = t_mid;
                } else {
                    t_low = t_mid;
                }
            }
            return Ok(Some(Duration::new(0.5 * (t_low + t_high))));
        }

        prev_t = curr_t;
    }

    Ok(None)
}

pub fn find_atmospheric_entry_event(
    elements: &OsculatingElements,
    mu: GravitationalParameter,
    entry_interface_radius: Length,
) -> RocketDomainResult<Option<(Angle, Duration)>> {
    let r_entry = entry_interface_radius.value();
    if r_entry <= 0.0 || !r_entry.is_finite() {
        return Ok(None);
    }

    if elements.periapsis_distance().value() >= r_entry {
        return Ok(None);
    }

    let Some((nu1, nu2)) = true_anomaly_at_radius(elements, entry_interface_radius) else {
        return Ok(None);
    };

    let nu_curr = elements.true_anomaly().value().rem_euclid(TAU);
    let nu_entry = if nu_curr < nu1.value() {
        nu2
    } else if nu_curr <= nu2.value() {
        nu2
    } else {
        nu2
    };

    let dt = time_between_true_anomalies(elements, mu, elements.true_anomaly(), nu_entry)?;
    Ok(Some((nu_entry, dt)))
}

pub fn compute_conic_patches(
    vehicle_id: Uuid,
    initial_state: (Position, VelocityVector),
    current_reference_body: &CelestialBodySoi,
    current_mu: GravitationalParameter,
    candidate_soi_bodies: &[CelestialBodySoi],
    current_universe_epoch: Duration,
    max_patches: usize,
    max_lookahead: Duration,
) -> RocketDomainResult<Vec<TrajectoryPatch>> {
    let mut patches = Vec::new();
    let mut curr_body = current_reference_body.clone();
    let mut curr_mu = current_mu;
    let mut curr_state = initial_state;
    let mut curr_epoch = current_universe_epoch;
    let lookahead_limit = current_universe_epoch + max_lookahead;

    let default_vehicle_props = AerocaptureVehicleProperties::new(
        Mass::new(5000.0),
        10.0,
        0.5,
        0.0,
        Length::new(0.5),
        HeatFlux::new(5.0e6),
        10.0,
    );

    while patches.len() < max_patches && curr_epoch.value() < lookahead_limit.value() {
        let elements = cartesian_to_osculating_elements(curr_state.0, curr_state.1, curr_mu)?;

        let exit_event = find_soi_exit_event(&elements, curr_mu, curr_body.soi_radius())?;

        let mut next_encounter: Option<(CelestialBodySoi, Duration)> = None;
        let remaining_time = lookahead_limit - curr_epoch;
        let search_window = match exit_event {
            Some((_, dt_exit)) => Duration::new(dt_exit.value().min(remaining_time.value())),
            None => remaining_time,
        };

        for body in candidate_soi_bodies {
            if body.id() == curr_body.id() {
                continue;
            }
            if body.parent_id() == Some(curr_body.id()) {
                if let Some(dt_enc) = find_body_encounter(
                    curr_state.0,
                    curr_state.1,
                    curr_mu,
                    body,
                    search_window,
                    100,
                )? {
                    if next_encounter.as_ref().map_or(true, |(_, dt)| dt_enc.value() < dt.value()) {
                        next_encounter = Some((body.clone(), dt_enc));
                    }
                }
            }
        }

        if let Some((target_body, dt_enc)) = next_encounter {
            let end_epoch = curr_epoch + dt_enc;
            let patch = TrajectoryPatch::from_osculating_elements(
                Uuid::new_v4(),
                vehicle_id,
                curr_body.id(),
                curr_epoch,
                Some(end_epoch),
                &elements,
                curr_mu,
            )?;
            patches.push(patch);

            let (r_enc, v_enc) =
                propagate_universal_state_vectors(curr_state.0, curr_state.1, curr_mu, dt_enc)?;
            let (r_new, v_new) = transform_state_to_new_soi(
                r_enc,
                v_enc,
                target_body.position(),
                VelocityVector::zero(),
            );

            curr_state = (r_new, v_new);
            curr_epoch = end_epoch;
            curr_mu = GravitationalParameter::new(
                astronomicon_core::units::constants::GRAVITATIONAL_CONSTANT
                    * target_body.mass().value(),
            );
            curr_body = target_body;
            continue;
        }

        if curr_body.has_atmosphere() {
            if let Some(r_entry) = curr_body.atmospheric_entry_radius() {
                let curr_r = curr_state.0.raw().magnitude();
                if curr_r > r_entry.value() {
                    if let Some((_, dt_entry)) = find_atmospheric_entry_event(&elements, curr_mu, r_entry)? {
                        if dt_entry.value() < search_window.value() {
                            let entry_epoch = curr_epoch + dt_entry;
                            let pre_entry_patch = TrajectoryPatch::from_osculating_elements(
                                Uuid::new_v4(),
                                vehicle_id,
                                curr_body.id(),
                                curr_epoch,
                                Some(entry_epoch),
                                &elements,
                                curr_mu,
                            )?;
                            patches.push(pre_entry_patch);

                            let (r_entry_pos, v_entry_vel) =
                                propagate_universal_state_vectors(curr_state.0, curr_state.1, curr_mu, dt_entry)?;

                            let atm_params = AtmosphericModelParameters::new(
                                curr_body.atmosphere_surface_density.unwrap_or(astronomicon_core::units::Density::new(1.225)),
                                curr_body.atmosphere_scale_height.unwrap_or(Length::new(8500.0)),
                                r_entry,
                                curr_body.body_radius,
                                curr_mu,
                                None,
                            );

                            let pass_res = simulate_atmospheric_pass(
                                r_entry_pos,
                                v_entry_vel,
                                entry_epoch,
                                &atm_params,
                                &default_vehicle_props,
                                Duration::new(7200.0),
                                Duration::new(0.5),
                            )?;

                            if let Ok(chebyshev_data) = pass_res.to_low_thrust_patch_data(7) {
                                let exit_t = pass_res.exit_epoch.unwrap_or(entry_epoch + Duration::new(600.0));
                                let aero_patch = TrajectoryPatch::new_low_thrust(
                                    Uuid::new_v4(),
                                    vehicle_id,
                                    curr_body.id(),
                                    entry_epoch,
                                    exit_t,
                                    curr_mu,
                                    chebyshev_data,
                                )?;
                                patches.push(aero_patch);
                            }

                            match pass_res.outcome {
                                AerocaptureOutcome::Captured { post_pass_elements, exit_epoch }
                                | AerocaptureOutcome::Escaped { exit_elements: post_pass_elements, exit_epoch } => {
                                    let conic_data = ConicPatchData::new(
                                        post_pass_elements.semi_major_axis(),
                                        post_pass_elements.eccentricity(),
                                        post_pass_elements.inclination(),
                                        post_pass_elements.longitude_of_ascending_node(),
                                        post_pass_elements.argument_of_periapsis(),
                                        post_pass_elements.true_anomaly(),
                                    )?;
                                    let post_patch = TrajectoryPatch::new_with_kind(
                                        Uuid::new_v4(),
                                        vehicle_id,
                                        curr_body.id(),
                                        exit_epoch,
                                        None,
                                        curr_mu,
                                        TrajectoryPatchKind::Conic(conic_data),
                                    )?;
                                    patches.push(post_patch);
                                    break;
                                }
                                _ => {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some((_, dt_exit)) = exit_event {
            let end_epoch = curr_epoch + dt_exit;
            let patch = TrajectoryPatch::from_osculating_elements(
                Uuid::new_v4(),
                vehicle_id,
                curr_body.id(),
                curr_epoch,
                Some(end_epoch),
                &elements,
                curr_mu,
            )?;
            patches.push(patch);

            let (r_exit, v_exit) =
                propagate_universal_state_vectors(curr_state.0, curr_state.1, curr_mu, dt_exit)?;

            let parent_body = candidate_soi_bodies
                .iter()
                .find(|b| Some(b.id()) == curr_body.parent_id())
                .cloned();

            if let Some(parent) = parent_body {
                let (r_parent, v_parent) = transform_state_from_old_soi(
                    r_exit,
                    v_exit,
                    curr_body.position(),
                    VelocityVector::zero(),
                );
                curr_state = (r_parent, v_parent);
                curr_epoch = end_epoch;
                curr_mu = GravitationalParameter::new(
                    astronomicon_core::units::constants::GRAVITATIONAL_CONSTANT
                        * parent.mass().value(),
                );
                curr_body = parent;
                continue;
            } else {
                break;
            }
        }

        let patch = TrajectoryPatch::from_osculating_elements(
            Uuid::new_v4(),
            vehicle_id,
            curr_body.id(),
            curr_epoch,
            None,
            &elements,
            curr_mu,
        )?;
        patches.push(patch);
        break;
    }

    Ok(patches)
}
