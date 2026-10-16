use crate::error::{RocketError, RocketResult};
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::domain::OrbitalParent;
use astronomicon_core::units::{Duration, Position};
use rocketcon_core::domain::VehiclePhysicalState;
use rocketcon_core::math::orbital::sphere_of_influence::laplace_sphere_of_influence_radius;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Copy, Debug)]
pub struct SoiCrossing {
    pub elapsed_seconds: f64,
    pub new_reference_id: Uuid,
    pub entering: bool,
}

#[derive(Clone, Copy)]
struct Boundary {
    body_id: Uuid,
    radius: f64,
    new_reference_id: Uuid,
    entering: bool,
}

fn candidates(catalog: &EphemerisCatalog, reference: Uuid) -> Vec<Boundary> {
    let mut result = Vec::new();
    let Some(primary_mass) = catalog.mass_of_body(reference) else {
        return result;
    };
    for body in catalog.planets() {
        if !matches!(
            body.orbital_parent(),
            OrbitalParent::Star(id) | OrbitalParent::Planet(id) if id == reference
        ) {
            continue;
        }
        let Some(elements) = body.orbital_elements() else {
            continue;
        };
        let radius = laplace_sphere_of_influence_radius(
            elements.semi_major_axis(),
            body.mass(),
            primary_mass,
        )
        .value();
        if radius.is_finite() && radius > 0.0 {
            result.push(Boundary {
                body_id: body.id(),
                radius,
                new_reference_id: body.id(),
                entering: true,
            });
        }
    }
    if let Some(active) = catalog.planets().iter().find(|body| body.id() == reference) {
        let parent_id = match active.orbital_parent() {
            OrbitalParent::Star(id) | OrbitalParent::Planet(id) => Some(id),
            _ => None,
        };
        if let Some(parent_id) = parent_id {
            if let (Some(parent_mass), Some(elements)) =
                (catalog.mass_of_body(parent_id), active.orbital_elements())
            {
                let radius = laplace_sphere_of_influence_radius(
                    elements.semi_major_axis(),
                    active.mass(),
                    parent_mass,
                )
                .value();
                if radius.is_finite() && radius > 0.0 {
                    result.push(Boundary {
                        body_id: reference,
                        radius,
                        new_reference_id: parent_id,
                        entering: false,
                    });
                }
            }
        }
    }
    result
}

pub fn first_crossing(
    catalog: &EphemerisCatalog,
    start_epoch: Duration,
    reference: Uuid,
    duration_seconds: f64,
    initial_position: Position,
    final_position: Position,
    mut position_at: impl FnMut(f64, &HashMap<Uuid, Position>) -> RocketResult<Position>,
) -> RocketResult<Option<SoiCrossing>> {
    if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
        return Ok(None);
    }
    let initial_bodies = catalog
        .positions(start_epoch)
        .map_err(|error| RocketError::Astronomicon(error.into()))?;
    let final_bodies = catalog
        .positions(start_epoch + Duration::new(duration_seconds))
        .map_err(|error| RocketError::Astronomicon(error.into()))?;
    let mut nearest: Option<SoiCrossing> = None;
    for boundary in candidates(catalog, reference) {
        let Some(origin) = initial_bodies.get(&boundary.body_id) else {
            continue;
        };
        let Some(destination) = final_bodies.get(&boundary.body_id) else {
            continue;
        };
        let start_relative = initial_position.raw() - origin.raw();
        let end_relative = final_position.raw() - destination.raw();
        let start_signed = start_relative.magnitude() - boundary.radius;
        let end_signed = end_relative.magnitude() - boundary.radius;
        if !start_signed.is_finite() || !end_signed.is_finite() {
            continue;
        }
        let crossing_expected = if boundary.entering {
            start_signed > 0.0
        } else {
            start_signed < 0.0
        };
        if !crossing_expected {
            continue;
        }
        let chord = end_relative - start_relative;
        let chord_sq = chord.dot(&chord);
        let closest_fraction = if chord_sq > 0.0 {
            (-start_relative.dot(&chord) / chord_sq).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let chord_distance = (start_relative + chord * closest_fraction).magnitude();
        if boundary.entering && end_signed > 0.0 && chord_distance > boundary.radius {
            continue;
        }
        if !boundary.entering && end_signed < 0.0 {
            continue;
        }
        let mut endpoints = vec![(0.0, start_signed)];
        if boundary.entering && closest_fraction > 0.0 && closest_fraction < 1.0 {
            let elapsed = duration_seconds * closest_fraction;
            endpoints.push((
                elapsed,
                signed_distance(
                    catalog,
                    start_epoch,
                    elapsed,
                    boundary.body_id,
                    boundary.radius,
                    &mut position_at,
                )?,
            ));
        }
        endpoints.push((duration_seconds, end_signed));
        let mut interval = None;
        for sample in endpoints.windows(2) {
            let before = sample[0];
            let after = sample[1];
            if (boundary.entering && before.1 > 0.0 && after.1 <= 0.0)
                || (!boundary.entering && before.1 < 0.0 && after.1 >= 0.0)
            {
                interval = Some((before.0, after.0));
                break;
            }
        }
        let Some((mut low, mut high)) = interval else {
            continue;
        };
        let resolution =
            (start_epoch.value().abs() * f64::EPSILON).max(duration_seconds * f64::EPSILON.sqrt());
        while high - low > resolution {
            let mid = (low + high) * 0.5;
            let signed = signed_distance(
                catalog,
                start_epoch,
                mid,
                boundary.body_id,
                boundary.radius,
                &mut position_at,
            )?;
            if (boundary.entering && signed <= 0.0) || (!boundary.entering && signed >= 0.0) {
                high = mid;
            } else {
                low = mid;
            }
        }
        let crossing = SoiCrossing {
            elapsed_seconds: high,
            new_reference_id: boundary.new_reference_id,
            entering: boundary.entering,
        };
        if nearest
            .as_ref()
            .is_none_or(|existing| crossing.elapsed_seconds < existing.elapsed_seconds)
        {
            nearest = Some(crossing);
        }
    }
    Ok(nearest)
}

fn signed_distance(
    catalog: &EphemerisCatalog,
    epoch: Duration,
    elapsed: f64,
    body: Uuid,
    radius: f64,
    position_at: &mut impl FnMut(f64, &HashMap<Uuid, Position>) -> RocketResult<Position>,
) -> RocketResult<f64> {
    let positions = catalog
        .positions(epoch + Duration::new(elapsed))
        .map_err(|error| RocketError::Astronomicon(error.into()))?;
    let origin = positions.get(&body).ok_or_else(|| {
        RocketError::Generic(format!("missing ephemeris for SOI boundary '{body}'"))
    })?;
    Ok((position_at(elapsed, &positions)?.raw() - origin.raw()).magnitude() - radius)
}

pub fn interpolate_world_position(
    previous: &VehiclePhysicalState,
    next: &VehiclePhysicalState,
    fraction: f64,
    duration_seconds: f64,
) -> Position {
    let t = fraction.clamp(0.0, 1.0);
    let t2 = t * t;
    let t3 = t2 * t;
    let p0 = previous.position().raw();
    let p1 = next.position().raw();
    let v0 = previous.velocity().raw();
    let v1 = next.velocity().raw();
    Position::from_raw(
        p0 * (2.0 * t3 - 3.0 * t2 + 1.0)
            + v0 * (duration_seconds * (t3 - 2.0 * t2 + t))
            + p1 * (-2.0 * t3 + 3.0 * t2)
            + v1 * (duration_seconds * (t3 - t2)),
    )
}
