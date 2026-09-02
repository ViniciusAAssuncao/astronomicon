use crate::error::RocketResult;
use astronomicon_app::ephemeris::resolve_system_positions;
use astronomicon_app::gravity::resolve_net_gravitational_acceleration;
use astronomicon_core::domain::{Planet, Star};
use astronomicon_core::math::gravity::{gravitational_parameter, j2_gravitational_acceleration};
use astronomicon_core::math::lagrange::orbital_plane_normal;
use astronomicon_core::math::reference_frames::inertial_to_body_fixed_position;
use astronomicon_core::math::rotation_state::{
    planet_rotation_angle_at_epoch, planet_rotation_axis_direction, resolve_planet_body_orientation,
    solstice_reference_direction,
};
use astronomicon_core::units::{
    AccelerationVector, Angle, Duration, Quaternion, Vector3,
};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{planet_repository, star_repository};
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
            let solstice_anomaly = planet
                .solstice_true_anomaly()
                .unwrap_or(Angle::new(0.0));
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

pub async fn resolve_vehicle_gravitational_acceleration(
    pool: &SqlitePool,
    environment: &EnvironmentSnapshot,
    physical_state: &VehiclePhysicalState,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<AccelerationVector> {
    let total_epoch = universe_epoch + at_epoch;

    let net_acc = resolve_net_gravitational_acceleration(
        pool,
        &environment.system_id,
        physical_state.position(),
        total_epoch,
    )
    .await?;

    let ref_id = physical_state.reference_body_id();

    let (mu, eq_radius, j2_opt, body_pos_fallback, orientation) = if ref_id == environment.planet.id() {
        let p = &environment.planet;
        (
            gravitational_parameter(p.mass()),
            p.equatorial_radius(),
            p.oblateness_j2(),
            environment.planet_position,
            planet_orientation(p, total_epoch),
        )
    } else if ref_id == environment.star.id() {
        let s = &environment.star;
        (
            gravitational_parameter(s.mass()),
            s.radius(),
            s.oblateness_j2(),
            environment.star_position,
            star_orientation(s, total_epoch),
        )
    } else if let Some(row) = planet_repository::get_by_id(pool, &ref_id).await? {
        let p = Planet::try_from(row)?;
        (
            gravitational_parameter(p.mass()),
            p.equatorial_radius(),
            p.oblateness_j2(),
            environment.planet_position,
            planet_orientation(&p, total_epoch),
        )
    } else if let Some(row) = star_repository::get_by_id(pool, &ref_id).await? {
        let s = Star::try_from(row)?;
        (
            gravitational_parameter(s.mass()),
            s.radius(),
            s.oblateness_j2(),
            environment.star_position,
            star_orientation(&s, total_epoch),
        )
    } else {
        return Ok(net_acc);
    };

    let j2 = match j2_opt {
        Some(val) if val.is_finite() && val != 0.0 => val,
        _ => return Ok(net_acc),
    };

    let r_eq = match eq_radius {
        Some(r) if r.value() > 0.0 && r.value().is_finite() => r,
        _ => return Ok(net_acc),
    };

    let positions = resolve_system_positions(pool, environment.system_id, total_epoch).await?;
    let body_pos = positions.get(&ref_id).copied().unwrap_or(body_pos_fallback);

    let vehicle_pos = physical_state.position();
    let body_fixed_pos = inertial_to_body_fixed_position(body_pos, orientation, vehicle_pos);

    let j2_acc_body = j2_gravitational_acceleration(mu, r_eq, j2, body_fixed_pos);
    let j2_acc_inertial_raw = orientation.rotate_vector(j2_acc_body.raw());
    let j2_acc_inertial = AccelerationVector::from_raw(j2_acc_inertial_raw);

    Ok(net_acc + j2_acc_inertial)
}