use crate::units::{Angle, Vector3};
use std::f64::consts::PI;

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
