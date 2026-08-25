use crate::units::{Acceleration, Density, DynamicViscosity, Length, Speed};

pub fn particle_terminal_velocity(
    gravity: Acceleration,
    particle_density: Density,
    fluid_density: Density,
    particle_radius: Length,
    dynamic_viscosity: DynamicViscosity,
) -> Speed {
    let g = gravity.value();
    let rho_p = particle_density.value();
    let rho_f = fluid_density.value();
    let r = particle_radius.value();
    let eta = dynamic_viscosity.value();

    if g <= 0.0
        || rho_p <= rho_f
        || rho_p <= 0.0
        || rho_f <= 0.0
        || r <= 0.0
        || eta <= 0.0
        || !g.is_finite()
        || !rho_p.is_finite()
        || !rho_f.is_finite()
        || !r.is_finite()
        || !eta.is_finite()
    {
        return Speed::new(0.0);
    }

    let delta_rho = rho_p - rho_f;
    let v_stokes = (2.0 / 9.0) * (delta_rho * g * r * r) / eta;
    let re_stokes = (2.0 * rho_f * v_stokes * r) / eta;

    if re_stokes < 1.0 {
        if !v_stokes.is_finite() || v_stokes <= 0.0 {
            return Speed::new(0.0);
        }
        return Speed::new(v_stokes);
    }

    let k = (8.0 * delta_rho * g * r) / (3.0 * rho_f);
    let v_newton = (k / 0.44).sqrt();
    let re_newton = (2.0 * rho_f * v_newton * r) / eta;

    if re_newton > 1000.0 {
        if !v_newton.is_finite() || v_newton <= 0.0 {
            return Speed::new(0.0);
        }
        return Speed::new(v_newton);
    }

    let mut low = 0.0;
    let mut high = v_stokes;
    let mut v = 0.5 * (low + high);

    for _ in 0..50 {
        let re = (2.0 * rho_f * v * r) / eta;
        let cd = if re <= 0.0 {
            f64::INFINITY
        } else {
            (24.0 / re) * (1.0 + 0.15 * re.powf(0.687))
        };
        let diff = v * v * cd - k;
        if diff < 0.0 {
            low = v;
        } else {
            high = v;
        }
        v = 0.5 * (low + high);
    }

    if !v.is_finite() || v <= 0.0 {
        Speed::new(0.0)
    } else {
        Speed::new(v)
    }
}

pub fn dynamic_aerosol_scale_height(
    gas_scale_height: Length,
    surface_gravity: Acceleration,
    eddy_diffusion_coefficient: f64,
    particle_density: Density,
    fluid_density: Density,
    particle_radius: Length,
    dynamic_viscosity: DynamicViscosity,
) -> Length {
    let h_gas = gas_scale_height.value();
    let k_zz = eddy_diffusion_coefficient;

    if h_gas <= 0.0 || k_zz <= 0.0 || !h_gas.is_finite() || !k_zz.is_finite() {
        return Length::new(0.0);
    }

    let v_term = particle_terminal_velocity(
        surface_gravity,
        particle_density,
        fluid_density,
        particle_radius,
        dynamic_viscosity,
    )
    .value();

    let denom = k_zz + v_term * h_gas;
    if denom <= 0.0 || !denom.is_finite() {
        return gas_scale_height;
    }

    let h_aero = (h_gas * k_zz) / denom;
    if !h_aero.is_finite() || h_aero <= 0.0 {
        Length::new(0.0)
    } else {
        Length::new(h_aero.min(h_gas))
    }
}

pub fn derived_aerosol_scale_height(
    surface_gravity: Acceleration,
    gas_scale_height: Length,
    atmospheric_density: Density,
) -> Length {
    let default_particle_density = Density::new(2500.0);
    let default_particle_radius = Length::new(1.5e-6);
    let default_viscosity = DynamicViscosity::new(1.81e-5);
    let default_k_zz = 1.2;

    dynamic_aerosol_scale_height(
        gas_scale_height,
        surface_gravity,
        default_k_zz,
        default_particle_density,
        atmospheric_density,
        default_particle_radius,
        default_viscosity,
    )
}

pub fn aerosol_density_at_altitude(
    surface_density: Density,
    altitude: Length,
    aerosol_scale_height: Length,
) -> Density {
    let rho_0 = surface_density.value();
    let z = altitude.value();
    let h_aero = aerosol_scale_height.value();

    if rho_0 <= 0.0 || !rho_0.is_finite() {
        return Density::new(0.0);
    }

    if z <= 0.0 {
        return surface_density;
    }

    if h_aero <= 0.0 || !h_aero.is_finite() || !z.is_finite() {
        return Density::new(0.0);
    }

    let exponent = -z / h_aero;
    if exponent < -700.0 {
        return Density::new(0.0);
    }

    let rho = rho_0 * exponent.exp();
    if !rho.is_finite() || rho <= 0.0 {
        Density::new(0.0)
    } else {
        Density::new(rho)
    }
}