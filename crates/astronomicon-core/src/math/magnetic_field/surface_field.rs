use crate::units::constants::VACUUM_PERMEABILITY;
use crate::units::{Length, MagneticDipoleMoment, MagneticFluxDensity};
use std::f64::consts::PI;

pub fn equatorial_surface_magnetic_field(
    dipole_moment: MagneticDipoleMoment,
    planet_radius: Length,
) -> MagneticFluxDensity {
    let m = dipole_moment.value();
    let r = planet_radius.value();

    if m <= 0.0 || r <= 0.0 || !m.is_finite() || !r.is_finite() {
        return MagneticFluxDensity::new(0.0);
    }

    let b = (VACUUM_PERMEABILITY * m) / (4.0 * PI * r.powi(3));
    MagneticFluxDensity::new(b)
}

pub fn polar_surface_magnetic_field(
    dipole_moment: MagneticDipoleMoment,
    planet_radius: Length,
) -> MagneticFluxDensity {
    let b_eq = equatorial_surface_magnetic_field(dipole_moment, planet_radius);
    MagneticFluxDensity::new(b_eq.value() * 2.0)
}
