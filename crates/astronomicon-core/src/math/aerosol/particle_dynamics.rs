use crate::math::aerosol::composite_properties::AtmosphericAerosolProperties;
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
    let v_term = (2.0 / 9.0) * (delta_rho * g * r * r) / eta;

    if !v_term.is_finite() || v_term <= 0.0 {
        Speed::new(0.0)
    } else {
        Speed::new(v_term)
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

pub fn aerosol_properties_at_altitude(
    surface_properties: &AtmosphericAerosolProperties,
    altitude: Length,
    aerosol_scale_height: Length,
) -> AtmosphericAerosolProperties {
    let z = altitude.value();
    let h_aero = aerosol_scale_height.value();

    if z <= 0.0 {
        return *surface_properties;
    }

    if h_aero <= 0.0 || !h_aero.is_finite() || !z.is_finite() {
        return AtmosphericAerosolProperties::new(
            Density::new(0.0),
            Density::new(0.0),
            Density::new(0.0),
            Density::new(0.0),
            surface_properties.asymmetry_factor_g(),
            0.0,
            0.0,
            surface_properties.angstrom_exponent(),
        );
    }

    let exponent = -z / h_aero;
    let factor = if exponent < -700.0 {
        0.0
    } else {
        exponent.exp()
    };

    AtmosphericAerosolProperties::new(
        Density::new(surface_properties.dust_density().value() * factor),
        Density::new(surface_properties.volcanic_density().value() * factor),
        Density::new(surface_properties.cloud_density().value() * factor),
        Density::new(surface_properties.total_density().value() * factor),
        surface_properties.asymmetry_factor_g(),
        surface_properties.base_extinction_coefficient() * factor,
        surface_properties.base_scattering_coefficient() * factor,
        surface_properties.angstrom_exponent(),
    )
}
