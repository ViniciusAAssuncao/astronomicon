use crate::chemistry::optics::GasOpticalProperties;
use crate::math::aerosol::AtmosphericAerosolProperties;
use crate::units::constants::{
    BOLTZMANN_CONSTANT, OPTICAL_REFERENCE_WAVELENGTH, STANDARD_ATMOSPHERE_PRESSURE, STP_TEMPERATURE,
};
use crate::units::{Angle, Density, Length, Pressure, Temperature, Vector3, Wavelength};
use std::f64::consts::PI;

pub fn size_parameter(particle_radius: Length, wavelength: Wavelength) -> f64 {
    let r = particle_radius.value();
    let lambda = wavelength.value();
    if r <= 0.0 || lambda <= 0.0 || !r.is_finite() || !lambda.is_finite() {
        0.0
    } else {
        (2.0 * PI * r) / lambda
    }
}

pub fn van_de_hulst_efficiencies(
    size_parameter: f64,
    refractive_index_real: f64,
    refractive_index_imag: f64,
) -> (f64, f64, f64) {
    let x = size_parameter;
    let nr = refractive_index_real.max(1.0);
    let ni = refractive_index_imag.max(0.0);

    if x <= 0.0 || !x.is_finite() || !nr.is_finite() || !ni.is_finite() {
        return (0.0, 0.0, 0.0);
    }

    let nr_sq = nr * nr;
    let ni_sq = ni * ni;

    let k_denom = (nr_sq - ni_sq + 2.0).powi(2) + 4.0 * nr_sq * ni_sq;
    let (k_real, k_imag) = if k_denom > 0.0 {
        let re = ((nr_sq - ni_sq - 1.0) * (nr_sq - ni_sq + 2.0) + 4.0 * nr_sq * ni_sq) / k_denom;
        let im = (6.0 * nr * ni) / k_denom;
        (re, im)
    } else {
        (0.0, 0.0)
    };

    let k_mag_sq = k_real * k_real + k_imag * k_imag;
    let q_sca_ray = (8.0 / 3.0) * x.powi(4) * k_mag_sq;
    let q_abs_ray = 4.0 * x * k_imag;

    let rho = 2.0 * x * (nr - 1.0);
    let xi = 4.0 * x * ni;
    let tan_beta = if nr > 1.0 { ni / (nr - 1.0) } else { 0.0 };
    let beta = tan_beta.atan();
    let cos_beta = beta.cos();

    let q_ext_vdh = if rho > 1.0e-4 {
        let term1 = 2.0;
        let exp_term = (-rho * tan_beta).exp();
        let cb_over_rho = cos_beta / rho;
        let term2 = 4.0 * exp_term * cb_over_rho * (rho - beta).sin();
        let term3 = 4.0 * exp_term * cb_over_rho * cb_over_rho * (rho - 2.0 * beta).cos();
        let term4 = 4.0 * cb_over_rho * cb_over_rho * (2.0 * beta).cos();
        (term1 - term2 - term3 + term4).max(0.0)
    } else {
        q_sca_ray + q_abs_ray
    };

    let q_abs_vdh = if xi > 1.0e-4 {
        let exp_xi = (-xi).exp();
        (1.0 + (exp_xi * (2.0 * xi + 1.0) - 1.0) / (2.0 * xi * xi)).clamp(0.0, 1.0)
    } else {
        (4.0 / 3.0) * xi * (1.0 - 0.375 * xi).max(0.0)
    };

    let q_sca_vdh = (q_ext_vdh - q_abs_vdh).max(0.0);

    let weight_vdh = (x.powi(4) / (1.0 + x.powi(4))).clamp(0.0, 1.0);
    let q_sca = (1.0 - weight_vdh) * q_sca_ray + weight_vdh * q_sca_vdh;
    let q_abs = (1.0 - weight_vdh) * q_abs_ray + weight_vdh * q_abs_vdh;
    let q_ext = q_sca + q_abs;

    (q_ext, q_sca, q_abs)
}

pub fn mie_asymmetry_factor(
    size_parameter: f64,
    refractive_index_real: f64,
    refractive_index_imag: f64,
) -> f64 {
    let x = size_parameter;
    let nr = refractive_index_real.max(1.0);
    let ni = refractive_index_imag.max(0.0);

    if x <= 0.0 || !x.is_finite() || !nr.is_finite() || !ni.is_finite() {
        return 0.0;
    }

    let size_weight = x * x / (1.0 + x * x);
    let refr_term = (1.0 - 0.45 / nr).clamp(0.0, 1.0);
    let absorp_term = 0.2 * (ni / (ni + 0.1));
    let g = size_weight * (refr_term - absorp_term);

    g.clamp(-0.999, 0.999)
}

pub fn mie_angstrom_exponent(
    particle_radius: Length,
    refractive_index_real: f64,
    refractive_index_imag: f64,
    reference_wavelength: Wavelength,
) -> f64 {
    let r = particle_radius.value();
    let lambda_0 = reference_wavelength.value();

    if r <= 0.0 || lambda_0 <= 0.0 || !r.is_finite() || !lambda_0.is_finite() {
        return 1.0;
    }

    let lambda_1 = Wavelength::new(lambda_0 * 0.9);
    let lambda_2 = Wavelength::new(lambda_0 * 1.1);

    let x1 = size_parameter(particle_radius, lambda_1);
    let x2 = size_parameter(particle_radius, lambda_2);

    let (_, q_sca1, _) =
        van_de_hulst_efficiencies(x1, refractive_index_real, refractive_index_imag);
    let (_, q_sca2, _) =
        van_de_hulst_efficiencies(x2, refractive_index_real, refractive_index_imag);

    if q_sca1 <= 1.0e-12 || q_sca2 <= 1.0e-12 {
        let x0 = size_parameter(particle_radius, reference_wavelength);
        return (4.0 / (1.0 + 0.6 * x0.powf(0.8))).clamp(0.0, 4.0);
    }

    let d_ln_q = (q_sca1 / q_sca2).ln();
    let d_ln_lambda = (lambda_1.value() / lambda_2.value()).ln();

    (-d_ln_q / d_ln_lambda).clamp(0.0, 4.0)
}

pub fn mass_optical_efficiencies(
    particle_radius: Length,
    particle_density: Density,
    refractive_index_real: f64,
    refractive_index_imag: f64,
    wavelength: Wavelength,
) -> (f64, f64, f64, f64, f64) {
    let r = particle_radius.value();
    let rho = particle_density.value();

    if r <= 0.0 || rho <= 0.0 || !r.is_finite() || !rho.is_finite() {
        return (0.0, 0.0, 0.0, 0.0, 1.0);
    }

    let x = size_parameter(particle_radius, wavelength);
    let (q_ext, q_sca, q_abs) =
        van_de_hulst_efficiencies(x, refractive_index_real, refractive_index_imag);
    let g = mie_asymmetry_factor(x, refractive_index_real, refractive_index_imag);
    let alpha = mie_angstrom_exponent(
        particle_radius,
        refractive_index_real,
        refractive_index_imag,
        wavelength,
    );

    let factor = 3.0 / (4.0 * rho * r);
    let k_ext = q_ext * factor;
    let k_sca = q_sca * factor;
    let k_abs = q_abs * factor;

    (k_ext, k_sca, k_abs, g, alpha)
}

pub fn rayleigh_scattering_cross_section(
    wavelength: Wavelength,
    refractivity_stp: f64,
    king_factor: f64,
) -> f64 {
    let lambda = wavelength.value();
    let delta = refractivity_stp;
    let f_k = if king_factor.is_finite() && king_factor > 0.0 {
        king_factor
    } else {
        1.0
    };

    if lambda <= 0.0 || delta <= 0.0 || !lambda.is_finite() || !delta.is_finite() {
        return 0.0;
    }

    let n = 1.0 + delta;
    let n2 = n * n;
    let n2_minus_1 = n2 - 1.0;

    let n_stp = STANDARD_ATMOSPHERE_PRESSURE / (BOLTZMANN_CONSTANT * STP_TEMPERATURE);
    let num = 8.0 * PI.powi(3) * n2_minus_1.powi(2) * f_k;
    let den = 3.0 * n_stp.powi(2) * lambda.powi(4);

    if den <= 0.0 || !den.is_finite() {
        return 0.0;
    }

    let sigma = num / den;
    if !sigma.is_finite() || sigma <= 0.0 {
        0.0
    } else {
        sigma
    }
}

pub fn molecular_number_density(pressure: Pressure, temperature: Temperature) -> f64 {
    let p = pressure.value();
    let t = temperature.value();

    if p <= 0.0 || t <= 0.0 || !p.is_finite() || !t.is_finite() {
        return 0.0;
    }

    let n = p / (BOLTZMANN_CONSTANT * t);
    if !n.is_finite() || n <= 0.0 { 0.0 } else { n }
}

pub fn rayleigh_scattering_coefficient(
    wavelength: Wavelength,
    refractivity_stp: f64,
    king_factor: f64,
    pressure: Pressure,
    temperature: Temperature,
) -> f64 {
    let sigma = rayleigh_scattering_cross_section(wavelength, refractivity_stp, king_factor);
    let n = molecular_number_density(pressure, temperature);
    let beta = sigma * n;

    if !beta.is_finite() || beta <= 0.0 {
        0.0
    } else {
        beta
    }
}

pub fn absorption_coefficient(
    gas_optical_properties: &GasOpticalProperties,
    wavelength: Wavelength,
    pressure: Pressure,
    temperature: Temperature,
) -> f64 {
    let sigma = gas_optical_properties.absorption_cross_section_at(wavelength);
    let n = molecular_number_density(pressure, temperature);
    let beta = sigma * n;

    if !beta.is_finite() || beta <= 0.0 {
        0.0
    } else {
        beta
    }
}

pub fn mie_scattering_coefficient(
    aerosol_properties: &AtmosphericAerosolProperties,
    wavelength: Wavelength,
    reference_wavelength: Wavelength,
) -> f64 {
    let beta_0 = aerosol_properties.base_scattering_coefficient();
    let lambda = wavelength.value();
    let lambda_0 = reference_wavelength.value();
    let alpha = aerosol_properties.angstrom_exponent();

    if beta_0 <= 0.0
        || lambda <= 0.0
        || lambda_0 <= 0.0
        || !beta_0.is_finite()
        || !lambda.is_finite()
        || !lambda_0.is_finite()
        || !alpha.is_finite()
    {
        return 0.0;
    }

    let ratio = lambda_0 / lambda;
    let beta = beta_0 * ratio.powf(alpha);

    if !beta.is_finite() || beta <= 0.0 {
        0.0
    } else {
        beta
    }
}

pub fn mie_scattering_coefficient_from_density(
    aerosol_density: Density,
    mass_scattering_efficiency: f64,
    wavelength: Wavelength,
    reference_wavelength: Wavelength,
    angstrom_exponent: f64,
) -> f64 {
    let rho = aerosol_density.value();
    let k_s = mass_scattering_efficiency;
    let lambda = wavelength.value();
    let lambda_0 = reference_wavelength.value();
    let alpha = angstrom_exponent;

    if rho <= 0.0
        || k_s <= 0.0
        || lambda <= 0.0
        || lambda_0 <= 0.0
        || !rho.is_finite()
        || !k_s.is_finite()
        || !lambda.is_finite()
        || !lambda_0.is_finite()
        || !alpha.is_finite()
    {
        return 0.0;
    }

    let beta_0 = rho * k_s;
    let ratio = lambda_0 / lambda;
    let beta = beta_0 * ratio.powf(alpha);

    if !beta.is_finite() || beta <= 0.0 {
        0.0
    } else {
        beta
    }
}

pub fn rayleigh_phase_function(scattering_angle: Angle) -> f64 {
    let theta = scattering_angle.value();
    if !theta.is_finite() {
        return 1.0 / (4.0 * PI);
    }
    let cos_theta = theta.cos();
    let val = (3.0 / (16.0 * PI)) * (1.0 + cos_theta * cos_theta);
    if !val.is_finite() || val < 0.0 {
        0.0
    } else {
        val
    }
}

pub fn rayleigh_phase_function_with_depolarization(
    scattering_angle: Angle,
    king_factor: f64,
) -> f64 {
    let theta = scattering_angle.value();
    let f_k = if king_factor.is_finite() && king_factor >= 1.0 {
        king_factor
    } else {
        1.0
    };

    if !theta.is_finite() {
        return 1.0 / (4.0 * PI);
    }

    let rho_n = ((6.0 * (f_k - 1.0)) / (3.0 + 7.0 * f_k)).clamp(0.0, 0.5);
    let cos_theta = theta.cos();
    let num = 1.0 + rho_n + (1.0 - rho_n) * cos_theta * cos_theta;
    let den = (4.0 * PI * (2.0 + rho_n)) / 3.0;

    if den <= 0.0 || !den.is_finite() {
        return 1.0 / (4.0 * PI);
    }

    let val = num / den;
    if !val.is_finite() || val < 0.0 {
        0.0
    } else {
        val
    }
}

pub fn henyey_greenstein_phase_function(scattering_angle: Angle, asymmetry_factor: f64) -> f64 {
    let theta = scattering_angle.value();
    let g = asymmetry_factor.clamp(-0.999, 0.999);

    if !theta.is_finite() {
        return 1.0 / (4.0 * PI);
    }

    let cos_theta = theta.cos();
    let denom_base = (1.0 + g * g - 2.0 * g * cos_theta).max(1e-7);
    let denom = 4.0 * PI * denom_base.powf(1.5);

    if denom <= 0.0 || !denom.is_finite() {
        return 1.0 / (4.0 * PI);
    }

    let num = 1.0 - g * g;
    let val = num / denom;

    if !val.is_finite() || val < 0.0 {
        0.0
    } else {
        val
    }
}

pub fn combined_scattering_phase_function(
    scattering_angle: Angle,
    rayleigh_coeff: f64,
    mie_coeff: f64,
    asymmetry_factor: f64,
) -> f64 {
    let b_r = rayleigh_coeff.max(0.0);
    let b_m = mie_coeff.max(0.0);
    let total = b_r + b_m;

    if total <= 0.0 || !total.is_finite() {
        return 1.0 / (4.0 * PI);
    }

    let p_r = rayleigh_phase_function(scattering_angle);
    let p_m = henyey_greenstein_phase_function(scattering_angle, asymmetry_factor);

    (b_r * p_r + b_m * p_m) / total
}

pub fn relative_airmass(zenith_angle: Angle) -> f64 {
    let z = zenith_angle.value().abs();
    if !z.is_finite() {
        return 1.0;
    }

    let half_pi = PI / 2.0;
    if z >= half_pi {
        return 40.0;
    }

    let cos_z = z.cos();
    let z_deg = z * (180.0 / PI);
    let diff = (96.07995 - z_deg).max(0.001);
    let denom = cos_z + 0.50572 * diff.powf(-1.6364);

    if denom <= 0.0 || !denom.is_finite() {
        40.0
    } else {
        (1.0 / denom).clamp(1.0, 40.0)
    }
}

pub fn vertical_optical_depth(
    rayleigh_coeff: f64,
    mie_coeff: f64,
    absorption_coeff: f64,
    scale_height: Length,
    aerosol_scale_height: Length,
) -> f64 {
    let b_r = rayleigh_coeff.max(0.0);
    let b_m = mie_coeff.max(0.0);
    let b_a = absorption_coeff.max(0.0);
    let h = scale_height.value().max(0.0);
    let h_aero = aerosol_scale_height.value().max(0.0);

    if !h.is_finite() || h <= 0.0 {
        return 0.0;
    }

    let total_extinction = (b_r + b_a) * h + b_m * h_aero;
    if !total_extinction.is_finite() || total_extinction <= 0.0 {
        0.0
    } else {
        total_extinction
    }
}

pub fn slant_optical_depth(vertical_optical_depth: f64, zenith_angle: Angle) -> f64 {
    if !vertical_optical_depth.is_finite() || vertical_optical_depth <= 0.0 {
        return 0.0;
    }
    let m = relative_airmass(zenith_angle);
    vertical_optical_depth * m
}

pub fn atmospheric_optical_depth(
    wavelength: Wavelength,
    gas_optical_properties: &GasOpticalProperties,
    pressure: Pressure,
    temperature: Temperature,
    aerosol_properties: &AtmosphericAerosolProperties,
    scale_height: Length,
    aerosol_scale_height: Length,
    zenith_angle: Angle,
) -> f64 {
    let b_r = rayleigh_scattering_coefficient(
        wavelength,
        gas_optical_properties.refractivity_stp(),
        gas_optical_properties.king_factor(),
        pressure,
        temperature,
    );
    let b_m = mie_scattering_coefficient(
        aerosol_properties,
        wavelength,
        Wavelength::new(OPTICAL_REFERENCE_WAVELENGTH),
    );
    let b_a = absorption_coefficient(gas_optical_properties, wavelength, pressure, temperature);

    let tau_0 = vertical_optical_depth(b_r, b_m, b_a, scale_height, aerosol_scale_height);
    slant_optical_depth(tau_0, zenith_angle)
}

pub fn atmospheric_transmittance(optical_depth: f64) -> f64 {
    if !optical_depth.is_finite() || optical_depth < 0.0 {
        return 1.0;
    }
    (-optical_depth).exp().clamp(0.0, 1.0)
}

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
