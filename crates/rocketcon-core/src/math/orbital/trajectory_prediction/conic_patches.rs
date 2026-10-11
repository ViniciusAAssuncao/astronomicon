use super::aerocapture_patch::{
    default_aerocapture_vehicle_properties, handle_atmospheric_entry_transition,
};
use super::events::{find_atmospheric_entry_event, find_body_encounter, find_soi_exit_event};
use super::multibody_regime::{build_cowell_config, is_in_multibody_regime};
use crate::constants::{
    CHEBYSHEV_DEFAULT_FIT_DEGREE, COWELL_DEFAULT_ABSOLUTE_TOLERANCE,
    COWELL_DEFAULT_INITIAL_STEP_S, COWELL_DEFAULT_RELATIVE_TOLERANCE,
    DEFAULT_BODY_ENCOUNTER_SEARCH_STEPS, MAX_N_BODY_PROPAGATION_WINDOW_S,
};
use crate::domain::TrajectoryPatch;
use crate::error::RocketDomainResult;
use crate::math::orbital::n_body::{
    n_body_trajectory_to_chebyshev_patch, propagate_n_body, NBodyPropagationConfig,
};
use crate::math::orbital::sphere_of_influence::{
    transform_state_from_old_soi, transform_state_to_new_soi, CelestialBodySoi,
};
use crate::math::orbital::universal::propagate_universal_state_vectors;
use crate::math::orbital::cartesian_to_osculating_elements;
use astronomicon_core::units::constants::GRAVITATIONAL_CONSTANT;
use astronomicon_core::units::{
    Duration, GravitationalParameter, Position, VelocityVector,
};
use uuid::Uuid;

fn try_multibody_step(
    vehicle_id: Uuid,
    curr_state: (Position, VelocityVector),
    curr_body: &CelestialBodySoi,
    curr_mu: GravitationalParameter,
    curr_epoch: Duration,
    lookahead_limit: Duration,
    candidate_soi_bodies: &[CelestialBodySoi],
    mass: astronomicon_core::units::Mass,
) -> Option<(TrajectoryPatch, (Position, VelocityVector), Duration)> {
    let cowell_config = build_cowell_config(curr_body, candidate_soi_bodies);
    let n_body_duration = Duration::new(
        (lookahead_limit - curr_epoch)
            .value()
            .min(MAX_N_BODY_PROPAGATION_WINDOW_S),
    );

    let n_body_config = NBodyPropagationConfig {
        initial_position: curr_state.0,
        initial_velocity: curr_state.1,
        initial_mass: mass,
        cowell_config,
        start_epoch: curr_epoch,
        duration: n_body_duration,
        initial_step: Duration::new(COWELL_DEFAULT_INITIAL_STEP_S),
        absolute_tolerance: COWELL_DEFAULT_ABSOLUTE_TOLERANCE,
        relative_tolerance: COWELL_DEFAULT_RELATIVE_TOLERANCE,
    };

    let n_body_res = propagate_n_body(&n_body_config).ok()?;
    let chebyshev_data = n_body_trajectory_to_chebyshev_patch(
        &n_body_res,
        CHEBYSHEV_DEFAULT_FIT_DEGREE,
        mass,
    )
    .ok()?;

    let end_t = n_body_res.final_epoch;
    let n_body_patch = TrajectoryPatch::new_low_thrust(
        Uuid::new_v4(),
        vehicle_id,
        curr_body.id(),
        curr_epoch,
        end_t,
        curr_mu,
        chebyshev_data,
    )
    .ok()?;

    let last_pt = n_body_res.points.last()?;
    let next_state = (last_pt.position, last_pt.velocity);

    Some((n_body_patch, next_state, end_t))
}

fn find_next_child_encounter(
    curr_state: (Position, VelocityVector),
    curr_body: &CelestialBodySoi,
    curr_mu: GravitationalParameter,
    search_window: Duration,
    candidate_soi_bodies: &[CelestialBodySoi],
) -> RocketDomainResult<Option<(CelestialBodySoi, Duration)>> {
    let mut next_encounter: Option<(CelestialBodySoi, Duration)> = None;

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
                DEFAULT_BODY_ENCOUNTER_SEARCH_STEPS,
            )? {
                if next_encounter
                    .as_ref()
                    .map_or(true, |(_, dt)| dt_enc.value() < dt.value())
                {
                    next_encounter = Some((body.clone(), dt_enc));
                }
            }
        }
    }

    Ok(next_encounter)
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

    let default_vehicle_props = default_aerocapture_vehicle_properties();

    while patches.len() < max_patches && curr_epoch.value() < lookahead_limit.value() {
        if is_in_multibody_regime(curr_state.0, &curr_body, candidate_soi_bodies) {
            if let Some((patch, next_state, end_t)) = try_multibody_step(
                vehicle_id,
                curr_state,
                &curr_body,
                curr_mu,
                curr_epoch,
                lookahead_limit,
                candidate_soi_bodies,
                default_vehicle_props.mass,
            ) {
                patches.push(patch);
                curr_state = next_state;
                curr_epoch = end_t;
                continue;
            }
        }

        let elements = cartesian_to_osculating_elements(curr_state.0, curr_state.1, curr_mu)?;
        let exit_event = find_soi_exit_event(&elements, curr_mu, curr_body.soi_radius())?;

        let remaining_time = lookahead_limit - curr_epoch;
        let search_window = match exit_event {
            Some((_, dt_exit)) => Duration::new(dt_exit.value().min(remaining_time.value())),
            None => remaining_time,
        };

        if let Some((target_body, dt_enc)) = find_next_child_encounter(
            curr_state,
            &curr_body,
            curr_mu,
            search_window,
            candidate_soi_bodies,
        )? {
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
                GRAVITATIONAL_CONSTANT * target_body.mass().value(),
            );
            curr_body = target_body;
            continue;
        }

        if curr_body.has_atmosphere() {
            if let Some(r_entry) = curr_body.atmospheric_entry_radius() {
                let curr_r = curr_state.0.raw().magnitude();
                if curr_r > r_entry.value() {
                    if let Some((_, dt_entry)) =
                        find_atmospheric_entry_event(&elements, curr_mu, r_entry)?
                    {
                        if dt_entry.value() < search_window.value() {
                            if let Some((entry_patches, should_break)) =
                                handle_atmospheric_entry_transition(
                                    vehicle_id,
                                    curr_state,
                                    &curr_body,
                                    curr_mu,
                                    curr_epoch,
                                    &elements,
                                    dt_entry,
                                    r_entry,
                                    &default_vehicle_props,
                                )?
                            {
                                patches.extend(entry_patches);
                                if should_break {
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
                    GRAVITATIONAL_CONSTANT * parent.mass().value(),
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