use crate::units::constants::VACUUM_PERMEABILITY;
use crate::units::{Length, MagneticDipoleMoment, Pressure};
use std::f64::consts::PI;

pub fn chapman_ferraro_radius(
    dipole_moment: MagneticDipoleMoment,
    stellar_wind_dynamic_pressure: Pressure,
) -> Length {
    let m = dipole_moment.value();
    let p_dyn = stellar_wind_dynamic_pressure.value();

    if m <= 0.0 || p_dyn <= 0.0 || !m.is_finite() || !p_dyn.is_finite() {
        return Length::new(0.0);
    }

    let numerator = VACUUM_PERMEABILITY * m * m;
    let denominator = 8.0 * PI * PI * p_dyn;
    let ratio = numerator / denominator;

    if !ratio.is_finite() || ratio <= 0.0 {
        return Length::new(0.0);
    }

    let r_mp = ratio.powf(1.0 / 6.0);
    Length::new(r_mp)
}

pub fn magnetopause_radius(
    dipole_moment: MagneticDipoleMoment,
    stellar_wind_dynamic_pressure: Pressure,
) -> Length {
    chapman_ferraro_radius(dipole_moment, stellar_wind_dynamic_pressure)
}
