use crate::units::Length;
use std::f64::consts::PI;

pub fn ground_solid_angle_factor(altitude: Length, planet_radius: Length) -> f64 {
    let r = planet_radius.value();
    let z = altitude.value().max(0.0);

    if r <= 0.0 || !r.is_finite() || !z.is_finite() {
        return 0.0;
    }

    let r_plus_z = r + z;
    if r_plus_z <= 0.0 {
        return 0.0;
    }

    let disc = z * (2.0 * r + z);
    let cos_horizon = disc.max(0.0).sqrt() / r_plus_z;
    let factor = 0.5 * (1.0 - cos_horizon.clamp(0.0, 1.0));

    factor.clamp(0.0, 0.5)
}

pub fn single_scattering_albedo(scattering_coefficient: f64, extinction_coefficient: f64) -> f64 {
    if extinction_coefficient <= 0.0 || !extinction_coefficient.is_finite() {
        return 0.0;
    }
    (scattering_coefficient / extinction_coefficient).clamp(0.0, 1.0)
}

pub fn multiple_scattering_transfer_factor(optical_depth: f64) -> f64 {
    let tau = optical_depth.max(0.0);
    if tau <= 0.0 || !tau.is_finite() {
        return 0.0;
    }
    if tau > 700.0 {
        1.0
    } else {
        let exp_neg_tau = (-tau).exp();
        (1.0 - exp_neg_tau * (1.0 + tau)).clamp(0.0, 1.0)
    }
}

pub fn ground_reflected_radiance(incident_irradiance: f64, ground_albedo: f64) -> f64 {
    let albedo = ground_albedo.clamp(0.0, 1.0);
    let irr = incident_irradiance.max(0.0);
    if !irr.is_finite() || irr <= 0.0 {
        0.0
    } else {
        (albedo / PI) * irr
    }
}

pub fn isotropic_multiple_scattering_source(
    direct_irradiance: f64,
    ground_irradiance: f64,
    ssa: f64,
    optical_depth: f64,
    multiple_scattering_factor: f64,
) -> f64 {
    let f_dir = direct_irradiance.max(0.0);
    let f_ground = ground_irradiance.max(0.0);

    if (f_dir <= 0.0 && f_ground <= 0.0) || ssa <= 0.0 || !ssa.is_finite() {
        return 0.0;
    }

    let f_ms_dir = multiple_scattering_transfer_factor(optical_depth)
        * multiple_scattering_factor.clamp(0.0, 5.0);
    let f_ms_ground = (if optical_depth > 700.0 {
        1.0
    } else {
        (1.0 - (-optical_depth).exp()).clamp(0.0, 1.0)
    }) * multiple_scattering_factor.clamp(0.0, 5.0);

    let j_dir = (ssa / (4.0 * PI)) * f_dir * f_ms_dir;
    let j_ground = (ssa / (4.0 * PI)) * f_ground * f_ms_ground;
    let order_2_factor = (ssa * f_ms_dir).clamp(0.0, 0.99);
    let denom = (1.0 - order_2_factor).max(0.01);

    let j_ms = (j_dir + j_ground) / denom;

    if !j_ms.is_finite() || j_ms < 0.0 {
        0.0
    } else {
        j_ms
    }
}
