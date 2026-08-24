use crate::units::constants::VACUUM_PERMEABILITY;
use crate::units::{
    Acceleration, AngularVelocity, Density, HeatFlux, Length, MagneticDipoleMoment,
};
use std::f64::consts::PI;

pub fn specific_buoyancy_flux(
    convective_heat_flux: HeatFlux,
    core_gravity: Acceleration,
    core_density: Density,
    thermal_expansion_coefficient: f64,
    specific_heat_capacity: f64,
) -> f64 {
    let q = convective_heat_flux.value();
    let g = core_gravity.value();
    let rho = core_density.value();
    let alpha = thermal_expansion_coefficient;
    let cp = specific_heat_capacity;

    if q <= 0.0
        || g <= 0.0
        || rho <= 0.0
        || alpha <= 0.0
        || cp <= 0.0
        || !q.is_finite()
        || !g.is_finite()
        || !rho.is_finite()
        || !alpha.is_finite()
        || !cp.is_finite()
    {
        return 0.0;
    }

    (alpha * g * q) / (rho * cp)
}

pub fn local_rossby_number(
    buoyancy_flux: f64,
    layer_thickness: Length,
    angular_velocity: AngularVelocity,
) -> f64 {
    let f = buoyancy_flux;
    let d = layer_thickness.value();
    let omega = angular_velocity.value().abs();

    if f <= 0.0
        || d <= 0.0
        || omega <= 0.0
        || !f.is_finite()
        || !d.is_finite()
        || !omega.is_finite()
    {
        return f64::INFINITY;
    }

    let convective_velocity = (f * d).cbrt();
    convective_velocity / (omega * d)
}

pub fn convective_magnetic_dipole_moment(
    core_radius: Length,
    core_density: Density,
    buoyancy_flux: f64,
    angular_velocity: AngularVelocity,
) -> MagneticDipoleMoment {
    let r_c = core_radius.value();
    let rho_c = core_density.value();
    let f = buoyancy_flux;
    let omega = angular_velocity.value().abs();

    if r_c <= 0.0
        || rho_c <= 0.0
        || f <= 0.0
        || omega <= 0.0
        || !r_c.is_finite()
        || !rho_c.is_finite()
        || !f.is_finite()
        || !omega.is_finite()
    {
        return MagneticDipoleMoment::new(0.0);
    }

    let layer_thickness = core_radius;
    let d = layer_thickness.value();
    let ro_l = local_rossby_number(f, layer_thickness, angular_velocity);
    let ro_crit = 0.12;

    let rotation_suppression = 1.0 / (1.0 + (ro_l / ro_crit).powi(4));

    let c_dip = 0.2;
    let density_term = (rho_c / VACUUM_PERMEABILITY).sqrt();
    let flux_term = (f * d).cbrt();
    let volume_scale = 4.0 * PI * r_c.powi(3);

    let dipole = c_dip * volume_scale * density_term * flux_term * rotation_suppression;

    if !dipole.is_finite() || dipole <= 0.0 {
        MagneticDipoleMoment::new(0.0)
    } else {
        MagneticDipoleMoment::new(dipole)
    }
}
