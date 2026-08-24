use crate::units::constants::STANDARD_GRAVITY;
use crate::units::{Acceleration, Density, Length, Speed};

pub fn dust_threshold_friction_velocity(
    gravity: Acceleration,
    atmospheric_density: Density,
    grain_density: Density,
    grain_diameter: Length,
) -> Speed {
    let g = gravity.value();
    let rho_a = atmospheric_density.value();
    let rho_p = grain_density.value();
    let d = grain_diameter.value();

    if g <= 0.0
        || rho_a <= 0.0
        || rho_p <= rho_a
        || d <= 0.0
        || !g.is_finite()
        || !rho_a.is_finite()
        || !rho_p.is_finite()
        || !d.is_finite()
    {
        return Speed::new(0.0);
    }

    let a_coeff = 0.118;
    let b_cohesion = 1.6e-4;
    let cohesion_term = 1.0 + b_cohesion / (rho_a * g * d);
    let density_ratio = (rho_p - rho_a) / rho_a;
    let u_star_t = a_coeff * (density_ratio * g * d * cohesion_term).sqrt();

    if !u_star_t.is_finite() || u_star_t <= 0.0 {
        Speed::new(0.0)
    } else {
        Speed::new(u_star_t)
    }
}

pub fn dust_threshold_surface_wind(gravity: Acceleration, atmospheric_density: Density) -> Speed {
    let grain_density = Density::new(2650.0);
    let grain_diameter = Length::new(1.0e-4);
    let u_star_t = dust_threshold_friction_velocity(
        gravity,
        atmospheric_density,
        grain_density,
        grain_diameter,
    );
    let drag_coeff: f64 = 0.003;
    let ratio = 1.0 / drag_coeff.sqrt();
    Speed::new(u_star_t.value() * ratio)
}

pub fn airborne_dust_density(
    surface_wind_speed: Speed,
    threshold_wind_speed: Speed,
    atmospheric_density: Density,
    surface_gravity: Acceleration,
    dust_availability_factor: f64,
    surface_coverage_fraction: f64,
    surface_humidity: f64,
) -> Density {
    let v = surface_wind_speed.value();
    let v_t = threshold_wind_speed.value();
    let rho_a = atmospheric_density.value();
    let g = surface_gravity.value();

    if v <= v_t
        || v_t <= 0.0
        || rho_a <= 0.0
        || g <= 0.0
        || !v.is_finite()
        || !v_t.is_finite()
        || !rho_a.is_finite()
        || !g.is_finite()
    {
        return Density::new(0.0);
    }

    let f_dust = dust_availability_factor.clamp(0.0, 1.0);
    let f_land = (1.0 - surface_coverage_fraction.clamp(0.0, 1.0)).max(0.0);
    let h = surface_humidity.clamp(0.0, 1.0);
    let f_moisture = (1.0 - h).powi(2);
    let suppression = f_dust * f_land * f_moisture;

    if suppression <= 0.0 || !suppression.is_finite() {
        return Density::new(0.0);
    }

    let delta_v = v - v_t;
    let normalized_excess = delta_v / (v_t + 0.1);
    let g_scale = (STANDARD_GRAVITY / g).clamp(0.1, 10.0);
    let c_dust = 1.5e-5;

    let rho_dust = c_dust * rho_a * normalized_excess * normalized_excess * g_scale * suppression;
    let clamped_dust = rho_dust.clamp(0.0, 0.05);

    if !clamped_dust.is_finite() {
        Density::new(0.0)
    } else {
        Density::new(clamped_dust)
    }
}