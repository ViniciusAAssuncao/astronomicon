use super::orbit_forecast_events::first_surface_intersection;
use crate::{RocketError, RocketResult};
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::math::gravity::gravitational_parameter;
use astronomicon_core::units::{Duration, Position, Vector3, VelocityVector};
use rocketcon_app::aeroespacial::first_crossing;
use rocketcon_core::math::orbital::state_properties::orbital_period_if_bound;
use rocketcon_core::math::orbital::{
    cartesian_to_osculating_elements, propagate_universal_state_vectors,
};
use uuid::Uuid;

const MAX_VERTICES: usize = 385;
pub(super) const MAX_ARCS: usize = 8;

#[derive(Debug, Clone, PartialEq)]
pub struct ForecastArc {
    pub reference_body_id: Uuid,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub points_m: Vec<[f64; 3]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForecastEvent {
    pub kind: u8,
    pub body_name: String,
    pub seconds: f64,
    pub position_m: [f64; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForecastBody {
    pub name: String,
    pub radius_m: f64,
    pub position_m: [f64; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForecastResult {
    pub arcs: Vec<ForecastArc>,
    pub events: Vec<ForecastEvent>,
    pub bodies: Vec<ForecastBody>,
    pub elapsed_seconds: f64,
    pub impact_seconds: Option<f64>,
}

pub(super) fn body_name(catalog: &EphemerisCatalog, id: Uuid) -> String {
    catalog
        .stars()
        .iter()
        .find(|item| item.id() == id)
        .map(|item| item.name().to_owned())
        .or_else(|| {
            catalog
                .planets()
                .iter()
                .find(|item| item.id() == id)
                .map(|item| item.name().to_owned())
        })
        .unwrap_or_default()
}

pub fn sample_patched_forecast(
    catalog: &EphemerisCatalog,
    epoch: Duration,
    first_reference: Uuid,
    position: Position,
    velocity: VelocityVector,
    horizon: f64,
) -> RocketResult<ForecastResult> {
    let initial = catalog.states(epoch)?;
    let first = initial.get(&first_reference).ok_or_else(|| {
        RocketError::Generic(format!(
            "forecast reference '{first_reference}' has no ephemeris"
        ))
    })?;
    let origin = first.position.raw();
    let mut bodies = Vec::new();
    if let Some(radius_m) = catalog.radius_of_body(first_reference) {
        bodies.push(ForecastBody {
            name: body_name(catalog, first_reference),
            radius_m,
            position_m: [0.0; 3],
        });
    }
    let mut result = ForecastResult {
        arcs: vec![ForecastArc {
            reference_body_id: first_reference,
            start_seconds: 0.0,
            end_seconds: 0.0,
            points_m: vec![as_array(position.raw() - origin)],
        }],
        events: Vec::new(),
        bodies,
        elapsed_seconds: 0.0,
        impact_seconds: None,
    };
    if !horizon.is_finite() || horizon <= 0.0 {
        return Ok(result);
    }
    let mut reference = first_reference;
    let mut center_position = origin;
    let mut display_center = Vector3::new(0.0, 0.0, 0.0);
    let mut relative_position = Position::from_raw(position.raw() - origin);
    let mut relative_velocity = VelocityVector::from_raw(velocity.raw() - first.velocity.raw());
    let mut elapsed = 0.0;
    let mut goal = horizon;
    for _ in 1..MAX_VERTICES {
        if elapsed >= goal {
            break;
        }
        let mu = catalog
            .mass_of_body(reference)
            .map(gravitational_parameter)
            .ok_or_else(|| {
                RocketError::Generic(format!("forecast reference '{reference}' has no mass"))
            })?;
        let radius = relative_position.raw().magnitude();
        if !radius.is_finite() || radius <= 0.0 {
            break;
        }
        if catalog
            .radius_of_body(reference)
            .is_some_and(|surface| radius <= surface)
        {
            result.impact_seconds = Some(elapsed);
            result.events.push(ForecastEvent {
                kind: 2,
                body_name: body_name(catalog, reference),
                seconds: elapsed,
                position_m: as_array(display_center + relative_position.raw()),
            });
            break;
        }
        let momentum = relative_position
            .raw()
            .cross(&relative_velocity.raw())
            .magnitude();
        let max_step = goal / 96.0;
        let angular_step = if momentum > 0.0 {
            (radius * radius * 0.075 / momentum).max(1.0)
        } else {
            max_step
        };
        let speed = relative_velocity.raw().magnitude();
        let local_step = if speed.is_finite() && speed > 0.0 {
            radius / (speed * 8.0)
        } else {
            max_step
        };
        let mut step = (goal - elapsed)
            .min(max_step)
            .min(angular_step)
            .min(local_step);
        let mut next = None;
        for _ in 0..8 {
            if let Ok(state) = propagate_universal_state_vectors(
                relative_position,
                relative_velocity,
                mu,
                Duration::new(step),
            ) {
                next = Some(state);
                break;
            }
            step *= 0.5;
        }
        let Some((next_position, next_velocity)) = next else {
            break;
        };
        let next_epoch = epoch + Duration::new(elapsed + step);
        let next_positions = catalog
            .positions(next_epoch)
            .map_err(|error| RocketError::Generic(error.to_string()))?;
        let center_end = next_positions.get(&reference).ok_or_else(|| {
            RocketError::Generic(format!("missing forecast center '{reference}'"))
        })?;
        let world_start = center_position + relative_position.raw();
        let world_end = center_end.raw() + next_position.raw();
        let soi = first_crossing(
            catalog,
            epoch + Duration::new(elapsed),
            reference,
            step,
            Position::from_raw(world_start),
            Position::from_raw(world_end),
            |dt, positions| {
                let (state, _) = propagate_universal_state_vectors(
                    relative_position,
                    relative_velocity,
                    mu,
                    Duration::new(dt),
                )?;
                let center = positions.get(&reference).ok_or_else(|| {
                    RocketError::Generic(format!("missing forecast body '{reference}'"))
                })?;
                Ok(Position::from_raw(center.raw() + state.raw()))
            },
        )?;
        let impact = catalog.radius_of_body(reference).and_then(|surface| {
            first_surface_intersection(relative_position, relative_velocity, mu, step, surface)
        });
        let impact_wins = impact.is_some_and(|time| {
            soi.as_ref()
                .is_none_or(|event| time <= event.elapsed_seconds)
        });
        let used = if impact_wins {
            impact.unwrap()
        } else {
            soi.as_ref().map_or(step, |event| event.elapsed_seconds)
        };
        if used <= 0.0 {
            break;
        }
        let propagated = if used == step {
            (next_position, next_velocity)
        } else {
            propagate_universal_state_vectors(
                relative_position,
                relative_velocity,
                mu,
                Duration::new(used),
            )?
        };
        let now = elapsed + used;
        let epoch_now = epoch + Duration::new(now);
        let frame = if soi.is_some() && !impact_wins {
            Some(catalog.states(epoch_now)?)
        } else {
            None
        };
        center_position = if used == step {
            center_end.raw()
        } else if let Some(states) = frame.as_ref() {
            states
                .get(&reference)
                .ok_or_else(|| {
                    RocketError::Generic(format!("missing crossing reference '{reference}'"))
                })?
                .position
                .raw()
        } else {
            catalog
                .positions(epoch_now)
                .map_err(|error| RocketError::Generic(error.to_string()))?
                .get(&reference)
                .ok_or_else(|| {
                    RocketError::Generic(format!("missing impact reference '{reference}'"))
                })?
                .raw()
        };
        let world = center_position + propagated.0.raw();
        let point = as_array(display_center + propagated.0.raw());
        let arc = result.arcs.last_mut().expect("forecast has an initial arc");
        arc.points_m.push(point);
        arc.end_seconds = now;
        elapsed = now;
        result.elapsed_seconds = now;
        if impact_wins {
            result.impact_seconds = Some(now);
            result.events.push(ForecastEvent {
                kind: 2,
                body_name: body_name(catalog, reference),
                seconds: now,
                position_m: point,
            });
            break;
        }
        relative_position = propagated.0;
        relative_velocity = propagated.1;
        if let Some(crossing) = soi {
            if result.arcs.len() >= MAX_ARCS {
                break;
            }
            let frame = frame.as_ref().expect("crossing has celestial states");
            let new_body = frame.get(&crossing.new_reference_id).ok_or_else(|| {
                RocketError::Generic(format!("missing target '{}'", crossing.new_reference_id))
            })?;
            let previous = frame.get(&reference).ok_or_else(|| {
                RocketError::Generic(format!("missing previous center '{reference}'"))
            })?;
            result.events.push(ForecastEvent {
                kind: if crossing.entering { 0 } else { 1 },
                body_name: body_name(
                    catalog,
                    if crossing.entering {
                        crossing.new_reference_id
                    } else {
                        reference
                    },
                ),
                seconds: now,
                position_m: point,
            });
            reference = crossing.new_reference_id;
            relative_position = Position::from_raw(world - new_body.position.raw());
            display_center = Vector3::new(point[0], point[1], point[2]) - relative_position.raw();
            if let Some(radius_m) = catalog.radius_of_body(reference) {
                result.bodies.push(ForecastBody {
                    name: body_name(catalog, reference),
                    radius_m,
                    position_m: as_array(display_center),
                });
            }
            relative_velocity = VelocityVector::from_raw(
                previous.velocity.raw() + propagated.1.raw() - new_body.velocity.raw(),
            );
            center_position = new_body.position.raw();
            if let Some(mu_new) = catalog.mass_of_body(reference).map(gravitational_parameter) {
                if let Ok(elements) =
                    cartesian_to_osculating_elements(relative_position, relative_velocity, mu_new)
                {
                    if let Some(period) = orbital_period_if_bound(&elements, mu_new) {
                        let one_revolution = now + period.value();
                        if one_revolution.is_finite()
                            && one_revolution > goal
                            && one_revolution <= 400.0 * 86400.0
                        {
                            goal = one_revolution;
                        }
                    }
                }
            }
            result.arcs.push(ForecastArc {
                reference_body_id: reference,
                start_seconds: now,
                end_seconds: now,
                points_m: vec![point],
            });
        }
    }
    Ok(result)
}

fn as_array(vector: Vector3) -> [f64; 3] {
    [vector.0, vector.1, vector.2]
}
