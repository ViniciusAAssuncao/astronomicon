use crate::units::constants::{GRAVITATIONAL_CONSTANT, SPEED_OF_LIGHT, THORNE_SPIN_LIMIT};
use crate::units::{Angle, Length, Mass};

pub fn gravitational_radius(mass: Mass) -> Length {
    let m = mass.value();
    if m <= 0.0 || !m.is_finite() {
        return Length::new(0.0);
    }
    let rg = (GRAVITATIONAL_CONSTANT * m) / (SPEED_OF_LIGHT * SPEED_OF_LIGHT);
    Length::new(rg)
}

pub fn schwarzschild_radius(mass: Mass) -> Length {
    Length::new(2.0 * gravitational_radius(mass).value())
}

pub fn event_horizon_radius(mass: Mass, dimensionless_spin: f64) -> Length {
    let rg = gravitational_radius(mass).value();
    if rg <= 0.0 {
        return Length::new(0.0);
    }
    let a_star = dimensionless_spin.clamp(0.0, THORNE_SPIN_LIMIT);
    let term = (1.0 - a_star * a_star).max(0.0).sqrt();
    Length::new(rg * (1.0 + term))
}

pub fn ergosphere_radius(mass: Mass, dimensionless_spin: f64, latitude: Angle) -> Length {
    let rg = gravitational_radius(mass).value();
    if rg <= 0.0 {
        return Length::new(0.0);
    }
    let a_star = dimensionless_spin.clamp(0.0, THORNE_SPIN_LIMIT);
    let sin_lat = latitude.value().sin();
    let term = (1.0 - a_star * a_star * sin_lat * sin_lat).max(0.0).sqrt();
    Length::new(rg * (1.0 + term))
}

pub fn photon_sphere_radius_prograde(mass: Mass, dimensionless_spin: f64) -> Length {
    let rg = gravitational_radius(mass).value();
    if rg <= 0.0 {
        return Length::new(0.0);
    }
    let a_star = dimensionless_spin.clamp(0.0, THORNE_SPIN_LIMIT);
    let term = (2.0 / 3.0) * (-a_star).acos();
    Length::new(2.0 * rg * (1.0 + term.cos()))
}

pub fn photon_sphere_radius_retrograde(mass: Mass, dimensionless_spin: f64) -> Length {
    let rg = gravitational_radius(mass).value();
    if rg <= 0.0 {
        return Length::new(0.0);
    }
    let a_star = dimensionless_spin.clamp(0.0, THORNE_SPIN_LIMIT);
    let term = (2.0 / 3.0) * a_star.acos();
    Length::new(2.0 * rg * (1.0 + term.cos()))
}

pub fn photon_sphere_radii(mass: Mass, dimensionless_spin: f64) -> (Length, Length) {
    (
        photon_sphere_radius_prograde(mass, dimensionless_spin),
        photon_sphere_radius_retrograde(mass, dimensionless_spin),
    )
}

pub fn isco_radius_prograde(mass: Mass, dimensionless_spin: f64) -> Length {
    let rg = gravitational_radius(mass).value();
    if rg <= 0.0 {
        return Length::new(0.0);
    }
    let a = dimensionless_spin.clamp(0.0, THORNE_SPIN_LIMIT);
    let z1 = 1.0 + (1.0 - a * a).cbrt() * ((1.0 + a).cbrt() + (1.0 - a).cbrt());
    let z2 = (3.0 * a * a + z1 * z1).sqrt();
    let term = ((3.0 - z1) * (3.0 + z1 + 2.0 * z2)).max(0.0).sqrt();
    let r_isco = rg * (3.0 + z2 - term);
    Length::new(r_isco)
}

pub fn isco_radius_retrograde(mass: Mass, dimensionless_spin: f64) -> Length {
    let rg = gravitational_radius(mass).value();
    if rg <= 0.0 {
        return Length::new(0.0);
    }
    let a = dimensionless_spin.clamp(0.0, THORNE_SPIN_LIMIT);
    let z1 = 1.0 + (1.0 - a * a).cbrt() * ((1.0 + a).cbrt() + (1.0 - a).cbrt());
    let z2 = (3.0 * a * a + z1 * z1).sqrt();
    let term = ((3.0 - z1) * (3.0 + z1 + 2.0 * z2)).max(0.0).sqrt();
    let r_isco = rg * (3.0 + z2 + term);
    Length::new(r_isco)
}

pub fn isco_radii(mass: Mass, dimensionless_spin: f64) -> (Length, Length) {
    (
        isco_radius_prograde(mass, dimensionless_spin),
        isco_radius_retrograde(mass, dimensionless_spin),
    )
}
