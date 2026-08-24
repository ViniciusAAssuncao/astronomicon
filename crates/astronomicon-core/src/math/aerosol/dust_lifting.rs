use crate::units::constants::STANDARD_GRAVITY;
use crate::units::{Acceleration, Density, DynamicViscosity, Length, Speed};

fn gamma_lanczos(z: f64) -> f64 {
    if z < 0.5 {
        std::f64::consts::PI / ((std::f64::consts::PI * z).sin() * gamma_lanczos(1.0 - z))
    } else {
        let z = z - 1.0;
        let c = [
            0.99999999999980993,
            676.5203681218851,
            -1259.1392167224028,
            771.32342877765313,
            -176.61502916214059,
            12.507343278686905,
            -0.138571095836524,
            9.9843695780195716e-6,
            1.5056327351493116e-7,
        ];
        let mut x = c[0];
        for i in 1..9 {
            x += c[i] / (z + (i as f64));
        }
        let t = z + 7.5;
        (2.0 * std::f64::consts::PI).sqrt() * t.powf(z + 0.5) * (-t).exp() * x
    }
}

pub fn particle_friction_reynolds_number(
    friction_velocity: Speed,
    grain_diameter: Length,
    kinematic_viscosity: f64,
) -> f64 {
    let u_star = friction_velocity.value();
    let d = grain_diameter.value();
    let nu = kinematic_viscosity;

    if u_star <= 0.0
        || d <= 0.0
        || nu <= 0.0
        || !u_star.is_finite()
        || !d.is_finite()
        || !nu.is_finite()
    {
        0.0
    } else {
        (u_star * d) / nu
    }
}

pub fn particle_reynolds_number(
    friction_velocity: Speed,
    grain_diameter: Length,
    kinematic_viscosity: f64,
) -> f64 {
    particle_friction_reynolds_number(friction_velocity, grain_diameter, kinematic_viscosity)
}

pub fn particle_friction_reynolds_number_from_dynamic(
    friction_velocity: Speed,
    grain_diameter: Length,
    atmospheric_density: Density,
    dynamic_viscosity: DynamicViscosity,
) -> f64 {
    let rho = atmospheric_density.value();
    let eta = dynamic_viscosity.value();
    if rho <= 0.0 || eta <= 0.0 || !rho.is_finite() || !eta.is_finite() {
        0.0
    } else {
        particle_friction_reynolds_number(friction_velocity, grain_diameter, eta / rho)
    }
}

pub fn dust_threshold_friction_velocity_with_viscosity(
    gravity: Acceleration,
    atmospheric_density: Density,
    grain_density: Density,
    grain_diameter: Length,
    kinematic_viscosity: f64,
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
    let gamma_cohesion = 1.6e-4;
    let density_ratio = (rho_p - rho_a) / rho_a;
    let cohesion_term = 1.0 + gamma_cohesion / ((rho_p - rho_a) * g * d * d);
    let k = a_coeff * a_coeff * density_ratio * g * d * cohesion_term;

    if !k.is_finite() || k <= 0.0 {
        return Speed::new(0.0);
    }

    let mut u = k.sqrt();
    let nu = if kinematic_viscosity.is_finite() && kinematic_viscosity > 0.0 {
        kinematic_viscosity
    } else {
        0.0
    };

    if nu > 0.0 {
        let b0 = 0.96;
        for _ in 0..20 {
            let re = (u * d) / nu;
            if re <= 1e-12 {
                break;
            }
            let next_u = (k * (1.0 + b0 / re)).sqrt();
            if (next_u - u).abs() < 1e-10 * u {
                u = next_u;
                break;
            }
            u = 0.5 * (u + next_u);
        }
    }

    if !u.is_finite() || u <= 0.0 {
        Speed::new(0.0)
    } else {
        Speed::new(u)
    }
}

pub fn dust_threshold_friction_velocity(
    gravity: Acceleration,
    atmospheric_density: Density,
    grain_density: Density,
    grain_diameter: Length,
) -> Speed {
    dust_threshold_friction_velocity_with_viscosity(
        gravity,
        atmospheric_density,
        grain_density,
        grain_diameter,
        0.0,
    )
}

pub fn optimal_saltation_diameter(
    gravity: Acceleration,
    atmospheric_density: Density,
    grain_density: Density,
    kinematic_viscosity: f64,
) -> Length {
    let g = gravity.value();
    let rho_a = atmospheric_density.value();
    let rho_p = grain_density.value();

    if g <= 0.0
        || rho_a <= 0.0
        || rho_p <= rho_a
        || !g.is_finite()
        || !rho_a.is_finite()
        || !rho_p.is_finite()
    {
        return Length::new(1.0e-4);
    }

    let gamma_cohesion = 1.6e-4;
    let d_inviscid = (gamma_cohesion / ((rho_p - rho_a) * g)).sqrt();

    let nu = if kinematic_viscosity.is_finite() && kinematic_viscosity > 0.0 {
        kinematic_viscosity
    } else {
        0.0
    };

    if nu <= 0.0 {
        if d_inviscid.is_finite() && d_inviscid > 0.0 {
            return Length::new(d_inviscid);
        } else {
            return Length::new(1.0e-4);
        }
    }

    let min_ln_d = (d_inviscid * 0.01).max(1.0e-7).ln();
    let max_ln_d = (d_inviscid * 100.0).min(0.05).max(d_inviscid * 2.0).ln();

    if min_ln_d >= max_ln_d {
        return Length::new(d_inviscid);
    }

    let eval = |ln_d: f64| -> f64 {
        let d = ln_d.exp();
        dust_threshold_friction_velocity_with_viscosity(
            gravity,
            atmospheric_density,
            grain_density,
            Length::new(d),
            nu,
        )
        .value()
    };

    let inv_phi2 = 0.381966011250105;
    let inv_phi = 0.618033988749895;

    let mut a = min_ln_d;
    let mut b = max_ln_d;
    let mut h = b - a;

    let mut c = a + inv_phi2 * h;
    let mut d = a + inv_phi * h;
    let mut fc = eval(c);
    let mut fd = eval(d);

    for _ in 0..60 {
        if fc < fd {
            b = d;
            d = c;
            fd = fc;
            h = b - a;
            c = a + inv_phi2 * h;
            fc = eval(c);
        } else {
            a = c;
            c = d;
            fc = fd;
            h = b - a;
            d = a + inv_phi * h;
            fd = eval(d);
        }
    }

    let best_ln_d = 0.5 * (a + b);
    let best_d = best_ln_d.exp();

    if !best_d.is_finite() || best_d <= 0.0 {
        Length::new(d_inviscid.max(1.0e-6))
    } else {
        Length::new(best_d)
    }
}

pub fn optimal_saltation_diameter_from_dynamic(
    gravity: Acceleration,
    atmospheric_density: Density,
    grain_density: Density,
    dynamic_viscosity: DynamicViscosity,
) -> Length {
    let rho = atmospheric_density.value();
    let eta = dynamic_viscosity.value();
    let nu = if rho > 0.0 && eta > 0.0 && rho.is_finite() && eta.is_finite() {
        eta / rho
    } else {
        0.0
    };
    optimal_saltation_diameter(gravity, atmospheric_density, grain_density, nu)
}

pub fn optimal_saltation_threshold_friction_velocity(
    gravity: Acceleration,
    atmospheric_density: Density,
    grain_density: Density,
    dynamic_viscosity: DynamicViscosity,
) -> (Length, Speed) {
    let rho = atmospheric_density.value();
    let eta = dynamic_viscosity.value();
    let nu = if rho > 0.0 && eta > 0.0 && rho.is_finite() && eta.is_finite() {
        eta / rho
    } else {
        0.0
    };
    let d_opt = optimal_saltation_diameter(gravity, atmospheric_density, grain_density, nu);
    let u_star_t = dust_threshold_friction_velocity_with_viscosity(
        gravity,
        atmospheric_density,
        grain_density,
        d_opt,
        nu,
    );
    (d_opt, u_star_t)
}

pub fn dust_threshold_surface_wind_with_params(
    gravity: Acceleration,
    atmospheric_density: Density,
    grain_density: Density,
    dynamic_viscosity: DynamicViscosity,
    drag_coefficient: Option<f64>,
) -> Speed {
    let cd = drag_coefficient.unwrap_or(0.003).clamp(1.0e-5, 0.1);
    let (_, u_star_t) = optimal_saltation_threshold_friction_velocity(
        gravity,
        atmospheric_density,
        grain_density,
        dynamic_viscosity,
    );
    let ratio = 1.0 / cd.sqrt();
    Speed::new(u_star_t.value() * ratio)
}

pub fn dust_threshold_surface_wind_with_viscosity(
    gravity: Acceleration,
    atmospheric_density: Density,
    dynamic_viscosity: DynamicViscosity,
) -> Speed {
    let default_grain_density = Density::new(2650.0);
    dust_threshold_surface_wind_with_params(
        gravity,
        atmospheric_density,
        default_grain_density,
        dynamic_viscosity,
        Some(0.003),
    )
}

pub fn dust_threshold_surface_wind(gravity: Acceleration, atmospheric_density: Density) -> Speed {
    let default_viscosity = DynamicViscosity::new(1.81e-5);
    dust_threshold_surface_wind_with_viscosity(gravity, atmospheric_density, default_viscosity)
}

pub fn weibull_scale_parameter(mean_wind_speed: Speed, shape_parameter: f64) -> f64 {
    let v_bar = mean_wind_speed.value();
    let k = shape_parameter.clamp(1.0, 5.0);

    if v_bar <= 0.0 || !v_bar.is_finite() {
        return 0.0;
    }

    let gamma_val = gamma_lanczos(1.0 + 1.0 / k);
    if gamma_val <= 0.0 || !gamma_val.is_finite() {
        0.0
    } else {
        v_bar / gamma_val
    }
}

pub fn wind_exceedance_probability(
    threshold_speed: Speed,
    mean_wind_speed: Speed,
    shape_parameter: f64,
) -> f64 {
    let v_t = threshold_speed.value();
    let lambda = weibull_scale_parameter(mean_wind_speed, shape_parameter);

    if v_t <= 0.0 || lambda <= 0.0 || !v_t.is_finite() || !lambda.is_finite() {
        return 0.0;
    }

    let k = shape_parameter.clamp(1.0, 5.0);
    let u = (v_t / lambda).powf(k);

    if u >= 50.0 {
        0.0
    } else {
        (-u).exp().clamp(0.0, 1.0)
    }
}

pub fn statistical_saltation_intensity(
    mean_wind_speed: Speed,
    threshold_wind_speed: Speed,
    shape_parameter: f64,
) -> f64 {
    let v_bar = mean_wind_speed.value();
    let v_t = threshold_wind_speed.value();

    if v_bar <= 0.0 || v_t <= 0.0 || !v_bar.is_finite() || !v_t.is_finite() {
        return 0.0;
    }

    let k = shape_parameter.clamp(1.0, 5.0);
    let gamma_val = gamma_lanczos(1.0 + 1.0 / k);
    if gamma_val <= 0.0 || !gamma_val.is_finite() {
        return 0.0;
    }

    let lambda = v_bar / gamma_val;
    if lambda <= 0.0 || !lambda.is_finite() {
        return 0.0;
    }

    let u0 = (v_t / lambda).powf(k);
    if u0 >= 40.0 || !u0.is_finite() {
        return 0.0;
    }

    let steps = 40;
    let max_u_offset = 16.0;
    let du = max_u_offset / (steps as f64);
    let inv_k = 1.0 / k;

    let mut sum = 0.0;
    for i in 0..steps {
        let u = u0 + ((i as f64) + 0.5) * du;
        let v = lambda * u.powf(inv_k);
        let delta_v = (v - v_t).max(0.0);
        let normalized_excess = delta_v / (v_t + 0.1);
        let intensity = normalized_excess * normalized_excess * (v / v_t);
        let weight = (-u).exp() * du;
        sum += intensity * weight;
    }

    if !sum.is_finite() || sum <= 0.0 {
        0.0
    } else {
        sum
    }
}

pub fn airborne_dust_density_with_gustiness(
    surface_wind_speed: Speed,
    threshold_wind_speed: Speed,
    atmospheric_density: Density,
    surface_gravity: Acceleration,
    dust_availability_factor: f64,
    surface_coverage_fraction: f64,
    surface_humidity: f64,
    shape_parameter: f64,
) -> Density {
    let rho_a = atmospheric_density.value();
    let g = surface_gravity.value();

    if rho_a <= 0.0 || g <= 0.0 || !rho_a.is_finite() || !g.is_finite() {
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

    let saltation_intensity =
        statistical_saltation_intensity(surface_wind_speed, threshold_wind_speed, shape_parameter);
    if saltation_intensity <= 0.0 {
        return Density::new(0.0);
    }

    let g_scale = (STANDARD_GRAVITY / g).clamp(0.1, 10.0);
    let c_dust = 1.5e-5;

    let rho_dust = c_dust * rho_a * saltation_intensity * g_scale * suppression;
    let clamped_dust = rho_dust.clamp(0.0, 0.05);

    if !clamped_dust.is_finite() {
        Density::new(0.0)
    } else {
        Density::new(clamped_dust)
    }
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
    airborne_dust_density_with_gustiness(
        surface_wind_speed,
        threshold_wind_speed,
        atmospheric_density,
        surface_gravity,
        dust_availability_factor,
        surface_coverage_fraction,
        surface_humidity,
        2.0,
    )
}