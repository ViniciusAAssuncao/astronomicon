use crate::error::{RocketError, RocketResult};
use astronomicon_app::ephemeris::resolve_system_positions;
use astronomicon_app::gravity::hierarchy_view::SystemHierarchy;
use astronomicon_core::domain::{Planet, Star};
use astronomicon_core::math::gravity::{
    gravitational_acceleration_at, gravitational_parameter, j2_gravitational_acceleration,
};
use astronomicon_core::math::lagrange::orbital_plane_normal;
use astronomicon_core::math::reference_frames::inertial_to_body_fixed_position;
use astronomicon_core::math::rotation_state::{
    planet_rotation_angle_at_epoch, planet_rotation_axis_direction,
    resolve_planet_body_orientation, solstice_reference_direction,
};
use astronomicon_core::units::{
    AccelerationVector, Angle, Duration, GravitationalParameter, Length, Mass, Position,
    Quaternion, Vector3,
};
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::VehiclePhysicalState;
use rocketcon_core::environment::EnvironmentSnapshot;

fn planet_orientation(planet: &Planet, total_epoch: Duration) -> Quaternion {
    let rot_period = match planet.rotation_period() {
        Some(p) if p.value() > 0.0 && p.value().is_finite() => p,
        _ => return Quaternion::identity(),
    };
    let prime_meridian = planet
        .prime_meridian_epoch_angle()
        .unwrap_or(Angle::new(0.0));
    let rot_angle = planet_rotation_angle_at_epoch(rot_period, prime_meridian, total_epoch);
    let spin_axis = match (planet.orbital_elements(), planet.obliquity()) {
        (Some(elements), Some(obliquity)) => {
            let normal = orbital_plane_normal(&elements);
            let solstice_anomaly = planet.solstice_true_anomaly().unwrap_or(Angle::new(0.0));
            let solstice_dir = solstice_reference_direction(&elements, solstice_anomaly);
            planet_rotation_axis_direction(normal, obliquity, solstice_dir)
        }
        _ => Vector3::new(0.0, 0.0, 1.0),
    };
    resolve_planet_body_orientation(spin_axis, rot_angle)
}

fn star_orientation(star: &Star, total_epoch: Duration) -> Quaternion {
    let rot_period = match star.rotation_period() {
        Some(p) if p.value() > 0.0 && p.value().is_finite() => p,
        _ => return Quaternion::identity(),
    };
    let rot_angle = planet_rotation_angle_at_epoch(rot_period, Angle::new(0.0), total_epoch);
    let spin_axis = match (star.orbital_elements(), star.obliquity()) {
        (Some(elements), Some(obliquity)) => {
            let normal = orbital_plane_normal(&elements);
            let solstice_dir = solstice_reference_direction(&elements, Angle::new(0.0));
            planet_rotation_axis_direction(normal, obliquity, solstice_dir)
        }
        _ => Vector3::new(0.0, 0.0, 1.0),
    };
    resolve_planet_body_orientation(spin_axis, rot_angle)
}

struct OblatenessField {
    center: Position,
    mu: GravitationalParameter,
    equatorial_radius: Length,
    j2: f64,
    orientation: Quaternion,
}

pub struct GravityField {
    sources: Vec<(Position, Mass)>,
    oblateness: Option<OblatenessField>,
}

impl GravityField {
    pub async fn load(
        pool: &SqlitePool,
        environment: &EnvironmentSnapshot,
        total_epoch: Duration,
    ) -> RocketResult<Self> {
        let hierarchy = SystemHierarchy::load(pool, &environment.system_id).await?;
        let positions = resolve_system_positions(pool, environment.system_id, total_epoch).await?;
        let mut sources = Vec::with_capacity(
            hierarchy.stars.len() + hierarchy.planets.len() + hierarchy.minor_planets.len(),
        );
        for (id, mass) in hierarchy
            .stars
            .iter()
            .map(|body| (body.id(), body.mass()))
            .chain(
                hierarchy
                    .planets
                    .iter()
                    .map(|body| (body.id(), body.mass())),
            )
            .chain(
                hierarchy
                    .minor_planets
                    .iter()
                    .map(|body| (body.id(), body.mass())),
            )
        {
            let position = positions.get(&id).copied().ok_or_else(|| {
                RocketError::Generic(format!("gravitational source '{}' has no ephemeris", id))
            })?;
            sources.push((position, mass));
        }

        let reference = environment.reference_id();
        let (mu, radius, j2, orientation) = if let Some(planet) = environment.planet.as_ref() {
            (
                gravitational_parameter(planet.mass()),
                planet.equatorial_radius(),
                planet.oblateness_j2(),
                planet_orientation(planet, total_epoch),
            )
        } else {
            let star = &environment.star;
            (
                gravitational_parameter(star.mass()),
                star.radius(),
                star.oblateness_j2(),
                star_orientation(star, total_epoch),
            )
        };
        let oblateness = match (radius, j2) {
            (Some(equatorial_radius), Some(j2))
                if equatorial_radius.value().is_finite()
                    && equatorial_radius.value() > 0.0
                    && j2.is_finite()
                    && j2 != 0.0 =>
            {
                let center = positions.get(&reference).copied().ok_or_else(|| {
                    RocketError::Generic(format!("reference '{}' has no ephemeris", reference))
                })?;
                Some(OblatenessField {
                    center,
                    mu,
                    equatorial_radius,
                    j2,
                    orientation,
                })
            }
            _ => None,
        };
        Ok(Self {
            sources,
            oblateness,
        })
    }

    pub fn acceleration_at(&self, position: Position) -> AccelerationVector {
        let point_mass = gravitational_acceleration_at(position, &self.sources);
        let Some(oblateness) = &self.oblateness else {
            return point_mass;
        };
        let body_fixed =
            inertial_to_body_fixed_position(oblateness.center, oblateness.orientation, position);
        let j2_body = j2_gravitational_acceleration(
            oblateness.mu,
            oblateness.equatorial_radius,
            oblateness.j2,
            body_fixed,
        );
        point_mass
            + AccelerationVector::from_raw(oblateness.orientation.rotate_vector(j2_body.raw()))
    }
}

pub async fn resolve_vehicle_gravitational_acceleration(
    pool: &SqlitePool,
    environment: &EnvironmentSnapshot,
    physical_state: &VehiclePhysicalState,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<AccelerationVector> {
    let field = GravityField::load(pool, environment, universe_epoch + at_epoch).await?;
    Ok(field.acceleration_at(physical_state.position()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use astronomicon_core::units::Vector3;

    #[test]
    fn empty_field_has_no_gravitational_force() {
        let field = GravityField {
            sources: Vec::new(),
            oblateness: None,
        };
        assert_eq!(
            field.acceleration_at(Position::zero()).raw(),
            Vector3::zero()
        );
    }

    #[test]
    fn gravity_varies_by_position_in_fixed_snapshot() {
        let mass = Mass::new(6.0e24);
        let center = Position::from_raw(Vector3::new(1.0e9, 0.0, 0.0));
        let field = GravityField {
            sources: vec![(center, mass)],
            oblateness: None,
        };
        let near = Position::from_raw(center.raw() + Vector3::new(7.0e6, 0.0, 0.0));
        let far = Position::from_raw(center.raw() + Vector3::new(14.0e6, 0.0, 0.0));
        let a_near = field.acceleration_at(near).raw();
        let a_far = field.acceleration_at(far).raw();
        assert!(a_near.0 < 0.0 && a_far.0 < 0.0);
        assert!(((a_near.magnitude() / a_far.magnitude()) - 4.0).abs() < 1e-12);
    }
}
