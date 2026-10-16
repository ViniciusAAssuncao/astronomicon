use crate::catalog_nbody_events::{
    ImpactCandidate, ImpactEvent, first_impact_in_segment, representable_epoch_step,
};
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::units::constants::GRAVITATIONAL_CONSTANT;
use astronomicon_core::units::{Duration, Position, VelocityVector};
use rocketcon_core::math::orbital::n_body::dop853::{
    Dop853Config, Dop853Error, Dop853Result, GravitySource,
    inertial_point_mass_derivative_from_sources, solve_dop853_with_event,
};
use uuid::Uuid;

pub struct CatalogTrajectory {
    pub integrated: Dop853Result<6>,
    pub source_ids: Vec<Uuid>,
    pub impact: Option<ImpactEvent>,
}

pub fn integrate_catalog_trajectory(
    catalog: &EphemerisCatalog,
    epoch: Duration,
    position: Position,
    velocity: VelocityVector,
    duration: Duration,
    included_bodies: Option<&[Uuid]>,
    config: Dop853Config<6>,
    cancelled: impl FnMut() -> bool,
) -> Result<CatalogTrajectory, Dop853Error> {
    integrate_catalog_trajectory_from_offset(
        catalog,
        epoch,
        0.0,
        position,
        velocity,
        duration,
        included_bodies,
        config,
        cancelled,
    )
}

pub fn integrate_catalog_trajectory_from_offset(
    catalog: &EphemerisCatalog,
    epoch: Duration,
    base_offset_s: f64,
    position: Position,
    velocity: VelocityVector,
    duration: Duration,
    included_bodies: Option<&[Uuid]>,
    config: Dop853Config<6>,
    cancelled: impl FnMut() -> bool,
) -> Result<CatalogTrajectory, Dop853Error> {
    let origin = position.raw();
    let velocity = velocity.raw();
    let initial = [0.0, 0.0, 0.0, velocity.0, velocity.1, velocity.2];
    let mut result = integrate_catalog_trajectory_local_frame(
        catalog,
        epoch,
        base_offset_s,
        position,
        initial,
        duration,
        included_bodies,
        config,
        cancelled,
    )?;
    for component in 0..3 {
        let offset = [origin.0, origin.1, origin.2][component];
        result.integrated.final_state[component] += offset;
        for segment in &mut result.integrated.segments {
            segment.start_state[component] += offset;
        }
    }
    Ok(result)
}

pub fn integrate_catalog_trajectory_local_frame(
    catalog: &EphemerisCatalog,
    epoch: Duration,
    base_offset_s: f64,
    origin: Position,
    initial: [f64; 6],
    duration: Duration,
    included_bodies: Option<&[Uuid]>,
    config: Dop853Config<6>,
    cancelled: impl FnMut() -> bool,
) -> Result<CatalogTrajectory, Dop853Error> {
    let span = duration.value();
    let frame = origin.raw();
    if ![frame.0, frame.1, frame.2]
        .iter()
        .all(|component| component.is_finite())
        || !epoch.value().is_finite()
        || !span.is_finite()
        || span <= 0.0
        || !base_offset_s.is_finite()
        || base_offset_s < 0.0
        || !(base_offset_s + span).is_finite()
        || !(epoch.value() + base_offset_s + span).is_finite()
        || !initial.iter().all(|component| component.is_finite())
    {
        return Err(Dop853Error::InvalidConfiguration);
    }
    let masses = catalog.gravitating_bodies();
    let mut candidates: Vec<ImpactCandidate> = masses
        .iter()
        .filter_map(|(id, _)| {
            catalog.radius_of_body(*id).map(|radius_m| ImpactCandidate {
                body_id: *id,
                radius_m,
            })
        })
        .collect();
    candidates.sort_by_key(|candidate| candidate.body_id);
    let selected = included_bodies.unwrap_or(&[]);
    if included_bodies.is_some() && selected.is_empty() {
        return Err(Dop853Error::InvalidGravitySource);
    }
    if selected
        .iter()
        .enumerate()
        .any(|(index, id)| selected[..index].contains(id))
    {
        return Err(Dop853Error::InvalidGravitySource);
    }
    let mut bodies: Vec<(Uuid, f64)> = masses
        .into_iter()
        .filter(|(id, _)| included_bodies.is_none() || selected.contains(id))
        .map(|(id, mass)| (id, mass.value() * GRAVITATIONAL_CONSTANT))
        .collect();
    if bodies.is_empty() || (included_bodies.is_some() && bodies.len() != selected.len()) {
        return Err(Dop853Error::InvalidGravitySource);
    }
    bodies.sort_by_key(|body| body.0);
    let mut sources: Vec<GravitySource> = bodies
        .iter()
        .map(|(id, mu)| GravitySource {
            body_id: *id,
            gravitational_parameter_m3_s2: *mu,
            position_m: [0.0; 3],
        })
        .collect();
    let requested_source_ids = bodies.iter().map(|(id, _)| *id).collect::<Vec<_>>();
    let requested_event_ids = candidates
        .iter()
        .map(|candidate| candidate.body_id)
        .collect::<Vec<_>>();
    let positions_at = |t: f64, events: bool| {
        if included_bodies.is_none() {
            catalog.positions_at_offset(epoch, base_offset_s + t)
        } else if events {
            catalog.positions_for_ids_at_offset(epoch, base_offset_s + t, &requested_event_ids)
        } else {
            catalog.positions_for_ids_at_offset(epoch, base_offset_s + t, &requested_source_ids)
        }
    };
    let start_positions =
        positions_at(0.0, true).map_err(|error| Dop853Error::Derivative(error.to_string()))?;
    for candidate in &candidates {
        let center = start_positions
            .get(&candidate.body_id)
            .ok_or_else(|| {
                Dop853Error::Derivative(format!(
                    "initial ephemeris missing for '{}'",
                    candidate.body_id
                ))
            })?
            .raw();
        let offset = center - origin.raw();
        let separation = ((initial[0] - offset.0).powi(2)
            + (initial[1] - offset.1).powi(2)
            + (initial[2] - offset.2).powi(2))
        .sqrt();
        if separation <= candidate.radius_m {
            return Err(Dop853Error::Derivative(format!(
                "initial state intersects body '{}'",
                candidate.body_id,
            )));
        }
    }
    let epoch_resolution = (representable_epoch_step(epoch)? * 0.001).max(1e-7);
    let mut impact: Option<ImpactEvent> = None;
    let origin = origin.raw();
    let integrated = solve_dop853_with_event(
        initial,
        span,
        config,
        |t, state| {
            let positions = positions_at(t, false)
                .map_err(|error| Dop853Error::Derivative(error.to_string()))?;
            for source in &mut sources {
                let point = positions.get(&source.body_id).ok_or_else(|| {
                    Dop853Error::Derivative(format!(
                        "missing ephemeris for body '{}' at relative epoch {t}",
                        source.body_id,
                    ))
                })?;
                let p = point.raw();
                source.position_m = [p.0 - origin.0, p.1 - origin.1, p.2 - origin.2];
            }
            inertial_point_mass_derivative_from_sources(state, &sources)
        },
        cancelled,
        |segment| {
            let mut cached = std::collections::HashMap::new();
            let found = first_impact_in_segment(
                segment,
                &candidates,
                epoch_resolution,
                24,
                |body_id, seconds| {
                    let key = seconds.to_bits();
                    if !cached.contains_key(&key) {
                        let positions = positions_at(seconds, true)
                            .map_err(|error| Dop853Error::Derivative(error.to_string()))?;
                        cached.insert(key, positions);
                    }
                    let point = cached
                        .get(&key)
                        .and_then(|positions| positions.get(&body_id))
                        .ok_or_else(|| {
                            Dop853Error::Derivative(format!(
                                "event ephemeris missing for '{body_id}'"
                            ))
                        })?
                        .raw();
                    Ok([point.0 - origin.0, point.1 - origin.1, point.2 - origin.2])
                },
            )?;
            if let Some(event) = found {
                impact = Some(event);
                Ok(Some(event.elapsed_seconds))
            } else {
                Ok(None)
            }
        },
    )?;
    Ok(CatalogTrajectory {
        integrated,
        source_ids: bodies.into_iter().map(|(id, _)| id).collect(),
        impact,
    })
}
