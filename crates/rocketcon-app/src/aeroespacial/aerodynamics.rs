use astronomicon_app::climate::atmosphere::resolve_atmospheric_profile_at_altitude;
use astronomicon_app::climate::circulation::resolve_wind_profile_at_latitude;
use astronomicon_app::climate::temperature::resolve_advective_surface_temperature;
use astronomicon_app::ephemeris::resolve_planet_orientation;
use astronomicon_app::error::AppResult;
use astronomicon_app::shape::effective_polar_radius_for_planet;
use astronomicon_core::domain::Planet;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::reference_frames::{ geodetic_altitude_and_normal, topocentric_basis };
use astronomicon_core::math::rotation::angular_velocity_from_rotation_period;
use astronomicon_core::math::thermodynamics::{ adiabatic_index_of_gas_mixture, speed_of_sound };
use astronomicon_core::units::constants::UNIVERSAL_GAS_CONSTANT;
use astronomicon_core::units::{
    Angle,
    AngularVelocityVector,
    Density,
    Duration,
    ForceVector,
    Length,
    Pressure,
    Speed,
    Vector3,
};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{ atmosphere_repository, planet_repository };
use rocketcon_core::domain::{ ComponentRecord, VehicleComponentEntry, VehiclePhysicalState };
use rocketcon_core::math::aerodynamics::{
    aerodynamic_drag_force,
    drag_coefficient_estimate,
    dynamic_pressure,
    local_atmospheric_relative_velocity,
    mach_number,
    vehicle_reference_cross_section_area,
};
use serde::{ Deserialize, Serialize };
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AerodynamicDiagnostic {
    pub altitude: Length,
    pub dynamic_pressure: Pressure,
    pub mach_number: f64,
    pub drag_coefficient: f64,
    pub reference_area_m2: f64,
    pub air_density: Density,
    pub speed_of_sound: Speed,
    pub relative_airspeed: Speed,
    pub drag_force: ForceVector,
    pub ambient_pressure: Pressure,
}

pub async fn resolve_vehicle_aerodynamics(
    pool: &SqlitePool,
    vehicle_physical_state: &VehiclePhysicalState,
    planet_id: Uuid,
    planet_inertial_position: Vector3,
    components: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
    universe_epoch: Duration,
    at_epoch: Duration
) -> AppResult<Option<AerodynamicDiagnostic>> {
    let atmosphere = match atmosphere_repository::get_by_planet_id(pool, &planet_id).await? {
        Some(atm) => atm,
        None => {
            return Ok(None);
        }
    };

    let planet_row = planet_repository
        ::get_by_id(pool, &planet_id).await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let eq_radius = planet.equatorial_radius().unwrap_or_else(|| Length::new(6371e3));
    let pol_radius = effective_polar_radius_for_planet(&planet);

    let vehicle_pos_raw = vehicle_physical_state.position().raw();
    let r_rel_inertial = vehicle_pos_raw - planet_inertial_position;

    let planet_orientation = resolve_planet_orientation(
        pool,
        planet_id,
        universe_epoch,
        at_epoch
    ).await?;
    let r_body = planet_orientation.inverse().rotate_vector(r_rel_inertial);

    let (altitude, normal_body) = geodetic_altitude_and_normal(
        eq_radius,
        pol_radius,
        astronomicon_core::units::Position::from_raw(r_body)
    );

    if altitude.value() < 0.0 {
        return Ok(None);
    }

    let lat_val = r_body.2.atan2((r_body.0 * r_body.0 + r_body.1 * r_body.1).sqrt());
    let latitude = Angle::new(lat_val);

    let surface_temperature = resolve_advective_surface_temperature(
        pool,
        planet_id,
        latitude,
        universe_epoch,
        at_epoch
    ).await?;

    let (p_alt, t_alt, rho_alt) = resolve_atmospheric_profile_at_altitude(
        pool,
        planet_id,
        surface_temperature,
        altitude
    ).await?;

    if rho_alt.value() <= 0.0 {
        return Ok(None);
    }

    let wind_diag = resolve_wind_profile_at_latitude(
        pool,
        planet_id,
        latitude,
        universe_epoch,
        at_epoch
    ).await?;

    let normal_inertial = planet_orientation.rotate_vector(normal_body);
    let spin_axis_inertial = planet_orientation.rotate_vector(Vector3::new(0.0, 0.0, 1.0));
    let (east, north, up) = topocentric_basis(normal_inertial, spin_axis_inertial);

    let rot_period = planet.rotation_period().unwrap_or_else(|| Duration::new(86400.0));
    let omega_mag = angular_velocity_from_rotation_period(rot_period);
    let planet_omega_inertial = AngularVelocityVector::from_raw(
        spin_axis_inertial * omega_mag.value()
    );

    let wind_topocentric = Vector3::new(
        wind_diag.surface_wind_u.value(),
        wind_diag.surface_wind_v.value(),
        0.0
    );

    let v_rel = local_atmospheric_relative_velocity(
        vehicle_physical_state.velocity(),
        planet_omega_inertial,
        r_rel_inertial,
        wind_topocentric,
        east,
        north,
        up
    );

    let v_rel_speed = v_rel.magnitude();

    let cp = atmosphere.mean_specific_heat_capacity()?;
    let molar_mass = atmosphere.mean_molar_mass()?;
    let specific_r = UNIVERSAL_GAS_CONSTANT / molar_mass.value();
    let gamma = adiabatic_index_of_gas_mixture(cp, specific_r);
    let sound_speed = speed_of_sound(t_alt, specific_r, gamma);

    let mach = mach_number(v_rel_speed, sound_speed);
    let cd = drag_coefficient_estimate(mach);
    let ref_area = vehicle_reference_cross_section_area(components, active_stages);

    let q = dynamic_pressure(rho_alt, v_rel_speed);
    let drag = aerodynamic_drag_force(q, cd, ref_area, v_rel.raw());

    Ok(
        Some(AerodynamicDiagnostic {
            altitude,
            dynamic_pressure: q,
            mach_number: mach,
            drag_coefficient: cd,
            reference_area_m2: ref_area,
            air_density: rho_alt,
            speed_of_sound: sound_speed,
            relative_airspeed: v_rel_speed,
            drag_force: drag,
            ambient_pressure: p_alt,
        })
    )
}
