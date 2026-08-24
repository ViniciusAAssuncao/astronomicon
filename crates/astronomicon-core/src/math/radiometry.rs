pub use crate::math::black_hole::schwarzschild_radius;
use crate::units::constants::STEFAN_BOLTZMANN_CONSTANT;
use crate::units::{
    Angle, Density, GravitationalParameter, Irradiance, Length, Luminosity, Mass, SolidAngle,
    Speed, Temperature, Vector3,
};
use std::f64::consts::PI;

pub fn stellar_luminosity(radius: Length, temperature: Temperature) -> Luminosity {
    if radius.value() <= 0.0 || temperature.value() <= 0.0 {
        return Luminosity::new(0.0);
    }
    let area = 4.0 * PI * radius.value() * radius.value();
    let t4 = temperature.value().powi(4);
    Luminosity::new(area * STEFAN_BOLTZMANN_CONSTANT * t4)
}

pub fn escape_velocity(mu: GravitationalParameter, radius: Length) -> Speed {
    if radius.value() <= 0.0 || mu.value() <= 0.0 {
        Speed::new(0.0)
    } else {
        Speed::new((2.0 * mu.value() / radius.value()).sqrt())
    }
}

pub fn mean_density(mass: Mass, radius: Length) -> Density {
    if radius.value() <= 0.0 || mass.value() <= 0.0 {
        Density::new(0.0)
    } else {
        let volume = (4.0 / 3.0) * PI * radius.value().powi(3);
        Density::new(mass.value() / volume)
    }
}

pub fn orbital_irradiance(luminosity: Luminosity, distance: Length) -> Irradiance {
    if distance.value() <= 0.0 || luminosity.value() <= 0.0 {
        Irradiance::new(0.0)
    } else {
        let area = 4.0 * PI * distance.value() * distance.value();
        Irradiance::new(luminosity.value() / area)
    }
}

pub fn equilibrium_temperature(
    star_temperature: Temperature,
    star_radius: Length,
    orbital_distance: Length,
    bond_albedo: f64,
) -> Temperature {
    let luminosity = stellar_luminosity(star_radius, star_temperature);
    let irradiance = orbital_irradiance(luminosity, orbital_distance);
    let absorbed = (1.0 - bond_albedo.clamp(0.0, 1.0)) * irradiance.value();
    let t4 = absorbed / (4.0 * STEFAN_BOLTZMANN_CONSTANT);
    Temperature::new(t4.max(0.0).powf(0.25))
}

pub fn stellar_angular_radius(stellar_radius: Length, distance: Length) -> Angle {
    let r = stellar_radius.value();
    let d = distance.value();

    if r <= 0.0 || d <= 0.0 || !r.is_finite() || !d.is_finite() {
        return Angle::new(0.0);
    }

    if r >= d {
        return Angle::new(PI / 2.0);
    }

    Angle::new((r / d).asin())
}

pub fn stellar_angular_diameter(stellar_radius: Length, distance: Length) -> Angle {
    Angle::new(2.0 * stellar_angular_radius(stellar_radius, distance).value())
}

pub fn stellar_solid_angle(angular_radius: Angle) -> SolidAngle {
    let theta = angular_radius.value();
    if theta <= 0.0 || !theta.is_finite() {
        return SolidAngle::new(0.0);
    }
    let clamped_theta = theta.clamp(0.0, PI);
    SolidAngle::new(2.0 * PI * (1.0 - clamped_theta.cos()))
}

pub fn stellar_solid_angle_from_distance(stellar_radius: Length, distance: Length) -> SolidAngle {
    let theta = stellar_angular_radius(stellar_radius, distance);
    stellar_solid_angle(theta)
}

pub fn stellar_limb_darkening(cos_theta: f64, linear_coefficient: f64) -> f64 {
    let mu = cos_theta.clamp(0.0, 1.0);
    let u = linear_coefficient.clamp(0.0, 1.0);
    let norm = 1.0 - u / 3.0;

    if norm <= 0.0 {
        1.0
    } else {
        (1.0 - u * (1.0 - mu)) / norm
    }
}

pub fn stellar_disk_sample_directions(
    center_dir: Vector3,
    angular_radius: Angle,
    sample_count: u32,
    limb_darkening_coeff: f64,
) -> Vec<(Vector3, f64)> {
    let d = center_dir.normalized();
    let theta_max = angular_radius.value();

    if theta_max <= 0.0 || sample_count <= 1 || !theta_max.is_finite() {
        return vec![(d, 1.0)];
    }

    let arbitrary = if d.0.abs() < 0.9 {
        Vector3::new(1.0, 0.0, 0.0)
    } else {
        Vector3::new(0.0, 1.0, 0.0)
    };
    let u_axis = d.cross(&arbitrary).normalized();
    let v_axis = d.cross(&u_axis).normalized();

    let n = sample_count;
    let golden_angle = PI * (3.0 - 5.0_f64.sqrt());
    let mut samples = Vec::with_capacity(n as usize);
    let mut weight_sum = 0.0;

    for i in 0..n {
        let frac = ((i as f64) + 0.5) / (n as f64);
        let rho = frac.sqrt();
        let r_angle = rho * theta_max;
        let phi = (i as f64) * golden_angle;

        let sin_r = r_angle.sin();
        let cos_r = r_angle.cos();
        let cos_phi = phi.cos();
        let sin_phi = phi.sin();

        let sample_dir = (d * cos_r + (u_axis * cos_phi + v_axis * sin_phi) * sin_r).normalized();
        let mu = (1.0 - rho * rho).max(0.0).sqrt();
        let w = stellar_limb_darkening(mu, limb_darkening_coeff);

        weight_sum += w;
        samples.push((sample_dir, w));
    }

    if weight_sum > 0.0 {
        for sample in &mut samples {
            sample.1 /= weight_sum;
        }
    }

    samples
}
