use astronomicon_core::math::eclipse::is_in_cylindrical_shadow;
use astronomicon_core::units::constants::SPEED_OF_LIGHT;
use astronomicon_core::units::{
    AccelerationVector, ForceVector, Irradiance, Length, Luminosity, Mass, Position, Pressure,
    Vector3,
};
use std::f64::consts::PI;

pub fn stellar_irradiance_at_distance(luminosity: Luminosity, distance: Length) -> Irradiance {
    let l = luminosity.value();
    let d = distance.value();
    if l <= 0.0 || d <= 0.0 || !l.is_finite() || !d.is_finite() {
        Irradiance::new(0.0)
    } else {
        Irradiance::new(l / (4.0 * PI * d * d))
    }
}

pub fn solar_radiation_pressure(irradiance: Irradiance) -> Pressure {
    let irr = irradiance.value();
    if irr <= 0.0 || !irr.is_finite() {
        Pressure::new(0.0)
    } else {
        Pressure::new(irr / SPEED_OF_LIGHT)
    }
}

pub fn srp_force(
    irradiance: Irradiance,
    effective_srp_area_m2: f64,
    cr: f64,
    sun_to_vehicle_direction: Vector3,
    is_shadowed: bool,
) -> ForceVector {
    if is_shadowed {
        return ForceVector::zero();
    }
    let p_rad = solar_radiation_pressure(irradiance).value();
    let a = effective_srp_area_m2;
    let coeff = cr.clamp(1.0, 2.0);

    if p_rad <= 0.0 || a <= 0.0 || !p_rad.is_finite() || !a.is_finite() {
        return ForceVector::zero();
    }

    let dir = sun_to_vehicle_direction.normalized();
    ForceVector::from_raw(dir * (p_rad * a * coeff))
}

pub fn srp_acceleration(
    vehicle_position: Position,
    star_position: Position,
    star_luminosity: Luminosity,
    planet_position: Position,
    planet_radius: Length,
    effective_srp_area_m2: f64,
    cr: f64,
    mass: Mass,
) -> AccelerationVector {
    let m = mass.value();
    if m <= 0.0 || !m.is_finite() {
        return AccelerationVector::zero();
    }

    let r_star_to_sc = vehicle_position.raw() - star_position.raw();
    let d_star = r_star_to_sc.magnitude();
    if d_star <= 1.0 || !d_star.is_finite() {
        return AccelerationVector::zero();
    }

    let is_eclipsed = is_in_cylindrical_shadow(
        vehicle_position,
        star_position,
        planet_position,
        planet_radius,
    );

    let irr = stellar_irradiance_at_distance(star_luminosity, Length::new(d_star));
    let f_srp = srp_force(irr, effective_srp_area_m2, cr, r_star_to_sc, is_eclipsed);

    AccelerationVector::from_raw(f_srp.raw() / m)
}
