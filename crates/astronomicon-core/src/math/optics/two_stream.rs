use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TwoStreamResult {
    pub reflectance: f64,
    pub transmittance: f64,
}

pub fn delta_eddington_two_stream(
    tau: f64,
    omega: f64,
    g: f64,
    mu0: f64,
    surface_albedo: f64,
) -> TwoStreamResult {
    let tau_c = tau.max(0.0);
    let omega_c = omega.clamp(0.0, 1.0);
    let g_c = g.clamp(-0.999, 0.999);
    let mu0_c = mu0.clamp(0.0001, 1.0);
    let a_s = surface_albedo.clamp(0.0, 1.0);

    let f = g_c * g_c;

    let denom_omega = 1.0 - omega_c * f;
    let tau_star = denom_omega * tau_c;

    let omega_star = if denom_omega > 0.0 {
        ((1.0 - f) * omega_c) / denom_omega
    } else {
        0.0
    };
    let omega_star = omega_star.clamp(0.0, 0.999999);

    let denom_g = 1.0 - f;
    let g_star = if denom_g > 0.0 {
        (g_c - f) / denom_g
    } else {
        0.0
    };
    let g_star = g_star.clamp(-0.999, 0.999);

    let gamma1 = 0.25 * (7.0 - omega_star * (4.0 + 3.0 * g_star));
    let gamma2 = -0.25 * (1.0 - omega_star * (4.0 - 3.0 * g_star));
    let gamma3 = 0.25 * (2.0 - 3.0 * g_star * mu0_c);
    let gamma4 = 1.0 - gamma3;

    let k = (gamma1 * gamma1 - gamma2 * gamma2).max(0.0).sqrt();
    let u = gamma2 / (gamma1 + k);

    let mut mu0_safe = mu0_c;
    let mut delta = 1.0 / (mu0_safe * mu0_safe) - k * k;
    if delta.abs() < 1e-6 {
        mu0_safe += 1e-5;
        delta = 1.0 / (mu0_safe * mu0_safe) - k * k;
    }

    let alpha1 = gamma1 * gamma4 + gamma2 * gamma3;
    let alpha2 = gamma1 * gamma3 + gamma2 * gamma4;

    let c_plus = omega_star * (alpha2 - gamma3 / mu0_safe) / delta;
    let c_minus = omega_star * (alpha1 + gamma4 / mu0_safe) / delta;

    let e_minus_k_tau = (-k * tau_star).exp();
    let e_minus_2k_tau = e_minus_k_tau * e_minus_k_tau;
    let e_minus_tau_mu0 = (-tau_star / mu0_safe).exp();

    let lhs_new = (1.0 - a_s * u) - u * e_minus_2k_tau * (u - a_s);
    let rhs = (a_s * mu0_safe + c_minus * a_s - c_plus) * e_minus_tau_mu0
        + c_minus * (u - a_s) * e_minus_k_tau;

    let k1_prime = if lhs_new.abs() > 1e-12 {
        rhs / lhs_new
    } else {
        0.0
    };

    let reflectance = k1_prime * e_minus_k_tau * (1.0 - u * u) - c_minus * u + c_plus;
    let transmittance = k1_prime * u * (1.0 - e_minus_2k_tau) - c_minus * e_minus_k_tau
        + c_minus * e_minus_tau_mu0;

    let r_fraction = (reflectance / mu0_safe).clamp(0.0, 1.0);
    let t_fraction = (transmittance / mu0_safe).clamp(0.0, 1.0);

    TwoStreamResult {
        reflectance: r_fraction,
        transmittance: t_fraction,
    }
}
