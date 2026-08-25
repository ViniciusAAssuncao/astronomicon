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

    if tau_c <= 1e-9 {
        return TwoStreamResult {
            reflectance: a_s,
            transmittance: 0.0,
        };
    }

    if omega_c <= 1e-9 {
        let e_dir = (-tau_c / mu0_c).exp();
        return TwoStreamResult {
            reflectance: a_s * e_dir * e_dir,
            transmittance: 0.0,
        };
    }

    let f = g_c * g_c;
    let denom_omega = (1.0 - omega_c * f).max(1e-12);
    let tau_star = denom_omega * tau_c;
    let omega_star = (((1.0 - f) * omega_c) / denom_omega).clamp(0.0, 0.9999999);
    let denom_g = (1.0 - f).max(1e-12);
    let g_star = ((g_c - f) / denom_g).clamp(-0.999, 0.999);

    let gamma1 = 0.25 * (7.0 - omega_star * (4.0 + 3.0 * g_star));
    let gamma2 = -0.25 * (1.0 - omega_star * (4.0 - 3.0 * g_star));
    let gamma3 = 0.25 * (2.0 - 3.0 * g_star * mu0_c);
    let gamma4 = 1.0 - gamma3;

    let k = (gamma1 * gamma1 - gamma2 * gamma2).max(0.0).sqrt();
    let u = gamma2 / (gamma1 + k);

    let mut mu0_safe = mu0_c;
    let mut denom_source = k * k - 1.0 / (mu0_safe * mu0_safe);
    if denom_source.abs() < 1e-5 {
        mu0_safe = if mu0_safe > 0.5 {
            mu0_safe - 1.5e-4
        } else {
            mu0_safe + 1.5e-4
        };
        denom_source = k * k - 1.0 / (mu0_safe * mu0_safe);
    }

    let alpha1 = gamma1 * gamma4 + gamma2 * gamma3;
    let alpha2 = gamma1 * gamma3 + gamma2 * gamma4;

    let c_plus = (omega_star * (alpha2 - gamma3 / mu0_safe)) / denom_source;
    let c_minus = (omega_star * (alpha1 + gamma4 / mu0_safe)) / denom_source;

    let e_k_tau = (-k * tau_star).exp();
    let e_2k_tau = e_k_tau * e_k_tau;
    let e_dir_tau = (-tau_star / mu0_safe).exp();

    let denom_k2 = (1.0 - u * u * e_2k_tau).max(1e-12);
    let k2_prime = (u * c_minus * e_k_tau - c_plus * e_dir_tau) / denom_k2;
    let k1 = -u * k2_prime * e_k_tau - c_minus;

    let f_up_0 = (u * k1 + k2_prime * e_k_tau + c_plus).max(0.0);
    let f_down_tau = (k1 * e_k_tau + u * k2_prime + c_minus * e_dir_tau).max(0.0);

    let r_0 = (f_up_0 / mu0_safe).clamp(0.0, 1.0);
    let t_diff_0 = (f_down_tau / mu0_safe).clamp(0.0, 1.0);
    let t_tot_0 = (t_diff_0 + e_dir_tau).clamp(0.0, 1.0);

    let denom_diffuse = (1.0 - u * u * e_2k_tau).max(1e-12);
    let r_bar = (u * (1.0 - e_2k_tau) / denom_diffuse).clamp(0.0, 1.0);
    let t_bar = ((1.0 - u * u) * e_k_tau / denom_diffuse).clamp(0.0, 1.0);

    let denom_mult = (1.0 - a_s * r_bar).max(1e-12);
    let r_combined = (r_0 + (a_s * t_tot_0 * t_bar) / denom_mult).clamp(0.0, 1.0);
    let t_diff_combined = ((t_diff_0 + a_s * r_bar * e_dir_tau) / denom_mult).clamp(0.0, 1.0);

    TwoStreamResult {
        reflectance: r_combined,
        transmittance: t_diff_combined,
    }
}
