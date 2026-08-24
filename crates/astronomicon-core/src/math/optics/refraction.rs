use crate::units::{Angle, Length, Vector3};
use std::f64::consts::PI;

pub fn refractive_index_at_altitude(
    surface_refractivity: f64,
    altitude: Length,
    scale_height: Length,
) -> f64 {
    let z = altitude.value();
    let h = scale_height.value();

    if surface_refractivity <= 0.0 || !surface_refractivity.is_finite() {
        return 1.0;
    }

    if h <= 0.0 || !h.is_finite() || z < 0.0 || !z.is_finite() {
        return 1.0 + surface_refractivity;
    }

    let exponent = -z / h;
    if exponent < -700.0 {
        1.0
    } else {
        1.0 + surface_refractivity * exponent.exp()
    }
}

pub fn spherical_snell_invariant(
    refractive_index: f64,
    radius: Length,
    zenith_angle: Angle,
) -> f64 {
    let n = refractive_index;
    let r = radius.value();
    let z = zenith_angle.value();

    if n <= 0.0 || r <= 0.0 || !n.is_finite() || !r.is_finite() || !z.is_finite() {
        0.0
    } else {
        n * r * z.sin().abs()
    }
}

pub fn zenith_angle_from_snell_invariant(
    snell_invariant: f64,
    refractive_index: f64,
    radius: Length,
) -> Option<Angle> {
    let inv = snell_invariant;
    let n = refractive_index;
    let r = radius.value();

    if inv < 0.0 || n <= 0.0 || r <= 0.0 || !inv.is_finite() || !n.is_finite() || !r.is_finite() {
        return None;
    }

    let ratio = inv / (n * r);
    if ratio > 1.0 {
        None
    } else {
        Some(Angle::new(ratio.clamp(0.0, 1.0).asin()))
    }
}

pub fn atmospheric_refraction_angle(
    apparent_zenith_angle: Angle,
    surface_refractivity: f64,
    scale_height: Length,
    planet_radius: Length,
) -> Angle {
    let z_a = apparent_zenith_angle.value().abs();
    let delta_0 = surface_refractivity;
    let h = scale_height.value();
    let r_p = planet_radius.value();

    if delta_0 <= 0.0
        || h <= 0.0
        || r_p <= 0.0
        || !delta_0.is_finite()
        || !h.is_finite()
        || !r_p.is_finite()
    {
        return Angle::new(0.0);
    }

    if z_a >= PI {
        return Angle::new(0.0);
    }

    let sin_z = z_a.sin();
    let cos_z = z_a.cos();
    let beta = h / r_p;
    let horizon_term = ((2.0 * beta) / PI).sqrt();
    let denom = (cos_z
        + (horizon_term * horizon_term + cos_z * cos_z * ((2.0 * beta) / PI)).sqrt())
    .max(1e-6);

    let r = delta_0 * (sin_z / denom);
    if !r.is_finite() || r <= 0.0 {
        Angle::new(0.0)
    } else {
        Angle::new(r)
    }
}

pub fn apparent_zenith_from_true(
    true_zenith_angle: Angle,
    surface_refractivity: f64,
    scale_height: Length,
    planet_radius: Length,
) -> Angle {
    let z_t = true_zenith_angle.value();
    if !z_t.is_finite() || surface_refractivity <= 0.0 {
        return true_zenith_angle;
    }

    let mut z_a = z_t
        - atmospheric_refraction_angle(
            true_zenith_angle,
            surface_refractivity,
            scale_height,
            planet_radius,
        )
        .value();
    let eps = 1e-6;

    for _ in 0..12 {
        let r = atmospheric_refraction_angle(
            Angle::new(z_a),
            surface_refractivity,
            scale_height,
            planet_radius,
        )
        .value();
        let f = z_a + r - z_t;
        if f.abs() < 1e-12 {
            break;
        }
        let r_plus = atmospheric_refraction_angle(
            Angle::new(z_a + eps),
            surface_refractivity,
            scale_height,
            planet_radius,
        )
        .value();
        let r_minus = atmospheric_refraction_angle(
            Angle::new(z_a - eps),
            surface_refractivity,
            scale_height,
            planet_radius,
        )
        .value();
        let df_dz = 1.0 + (r_plus - r_minus) / (2.0 * eps);
        let delta = f / df_dz;
        z_a -= delta;
    }

    Angle::new(z_a)
}

pub fn true_zenith_from_apparent(
    apparent_zenith_angle: Angle,
    surface_refractivity: f64,
    scale_height: Length,
    planet_radius: Length,
) -> Angle {
    let r = atmospheric_refraction_angle(
        apparent_zenith_angle,
        surface_refractivity,
        scale_height,
        planet_radius,
    );
    apparent_zenith_angle + r
}

pub fn refracted_sun_direction(
    geometric_sun_dir: Vector3,
    up_vector: Vector3,
    surface_refractivity: f64,
    scale_height: Length,
    planet_radius: Length,
) -> Vector3 {
    let s = geometric_sun_dir.normalized();
    let u = up_vector.normalized();

    if surface_refractivity <= 0.0 || !surface_refractivity.is_finite() {
        return s;
    }

    let cos_zt = s.dot(&u).clamp(-1.0, 1.0);
    let z_t = Angle::new(cos_zt.acos());
    let z_a = apparent_zenith_from_true(z_t, surface_refractivity, scale_height, planet_radius);

    let h = s - u * cos_zt;
    let h_mag = h.magnitude();

    if h_mag < 1e-12 {
        s
    } else {
        let h_unit = h / h_mag;
        let sin_za = z_a.value().sin();
        let cos_za = z_a.value().cos();
        (u * cos_za + h_unit * sin_za).normalized()
    }
}

pub fn unrefracted_sun_direction(
    apparent_sun_dir: Vector3,
    up_vector: Vector3,
    surface_refractivity: f64,
    scale_height: Length,
    planet_radius: Length,
) -> Vector3 {
    let s_app = apparent_sun_dir.normalized();
    let u = up_vector.normalized();

    if surface_refractivity <= 0.0 || !surface_refractivity.is_finite() {
        return s_app;
    }

    let cos_za = s_app.dot(&u).clamp(-1.0, 1.0);
    let z_a = Angle::new(cos_za.acos());
    let z_t = true_zenith_from_apparent(z_a, surface_refractivity, scale_height, planet_radius);

    let h = s_app - u * cos_za;
    let h_mag = h.magnitude();

    if h_mag < 1e-12 {
        s_app
    } else {
        let h_unit = h / h_mag;
        let sin_zt = z_t.value().sin();
        let cos_zt = z_t.value().cos();
        (u * cos_zt + h_unit * sin_zt).normalized()
    }
}
