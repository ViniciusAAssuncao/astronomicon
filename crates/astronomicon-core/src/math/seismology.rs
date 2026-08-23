use crate::domain::TectonicRegime;
use crate::math::hydrosphere::spherical_shell_volume;
use crate::units::{
    Acceleration, Density, HeatFlux, Length, Luminosity, Mass, Pressure, Speed,
};
use std::f64::consts::PI;

pub fn equilibrium_tidal_bulge_height(
    parent_mass: Mass,
    body_mass: Mass,
    body_radius: Length,
    semi_major_axis: Length,
    love_number_k2: f64,
) -> Length {
    let m_p = parent_mass.value();
    let m_b = body_mass.value();
    let r = body_radius.value();
    let a = semi_major_axis.value();

    if m_p <= 0.0
        || m_b <= 0.0
        || r <= 0.0
        || a <= 0.0
        || love_number_k2 <= 0.0
        || !m_p.is_finite()
        || !m_b.is_finite()
        || !r.is_finite()
        || !a.is_finite()
        || !love_number_k2.is_finite()
    {
        return Length::new(0.0);
    }

    let h2 = (5.0 / 3.0) * love_number_k2;
    let h_tide = h2 * (m_p / m_b) * (r.powi(4) / a.powi(3));

    if !h_tide.is_finite() || h_tide <= 0.0 {
        Length::new(0.0)
    } else {
        Length::new(h_tide)
    }
}

pub fn radial_tidal_stress_amplitude(
    eccentricity: f64,
    crust_density: Density,
    surface_gravity: Acceleration,
    tidal_bulge_height: Length,
) -> Pressure {
    let e = eccentricity;
    let rho = crust_density.value();
    let g = surface_gravity.value();
    let h_tide = tidal_bulge_height.value();

    if e <= 0.0
        || e >= 1.0
        || rho <= 0.0
        || g <= 0.0
        || h_tide <= 0.0
        || !e.is_finite()
        || !rho.is_finite()
        || !g.is_finite()
        || !h_tide.is_finite()
    {
        return Pressure::new(0.0);
    }

    let delta_sigma = 1.5 * e * rho * g * h_tide;

    if !delta_sigma.is_finite() || delta_sigma <= 0.0 {
        Pressure::new(0.0)
    } else {
        Pressure::new(delta_sigma)
    }
}

pub fn seismic_efficiency(yield_stress: Pressure, shear_modulus: Pressure) -> f64 {
    let sigma_y = yield_stress.value();
    let mu = shear_modulus.value();

    if sigma_y <= 0.0 || mu <= 0.0 || !sigma_y.is_finite() || !mu.is_finite() {
        return 0.001;
    }

    let eff = (0.1 * sigma_y) / (2.0 * mu);
    if !eff.is_finite() {
        return 0.001;
    }

    eff.clamp(0.001, 0.1)
}

pub fn tidal_seismic_energy_rate(
    total_tidal_power: Luminosity,
    planet_radius: Length,
    brittle_thickness: Length,
    seismic_efficiency: f64,
) -> Luminosity {
    let p_tide = total_tidal_power.value();
    let r = planet_radius.value();
    let z_b = brittle_thickness.value();
    let eta = seismic_efficiency.clamp(0.0, 1.0);

    if p_tide <= 0.0
        || r <= 0.0
        || z_b <= 0.0
        || !p_tide.is_finite()
        || !r.is_finite()
        || !z_b.is_finite()
    {
        return Luminosity::new(0.0);
    }

    let v_planet = (4.0 / 3.0) * PI * r.powi(3);
    let v_brittle = spherical_shell_volume(planet_radius, brittle_thickness, 1.0);

    if v_planet <= 0.0 || !v_planet.is_finite() {
        return Luminosity::new(0.0);
    }

    let volume_fraction = (v_brittle / v_planet).clamp(0.0, 1.0);
    let power = p_tide * volume_fraction * eta;

    if !power.is_finite() || power <= 0.0 {
        Luminosity::new(0.0)
    } else {
        Luminosity::new(power)
    }
}

pub fn thermal_contraction_strain_rate(
    surface_heat_flux: HeatFlux,
    planet_mass: Mass,
    planet_radius: Length,
    thermal_expansion: f64,
    specific_heat_capacity: f64,
) -> f64 {
    let q = surface_heat_flux.value();
    let m = planet_mass.value();
    let r = planet_radius.value();
    let alpha = thermal_expansion;
    let cp = specific_heat_capacity;

    if q <= 0.0
        || m <= 0.0
        || r <= 0.0
        || alpha <= 0.0
        || cp <= 0.0
        || !q.is_finite()
        || !m.is_finite()
        || !r.is_finite()
        || !alpha.is_finite()
        || !cp.is_finite()
    {
        return 0.0;
    }

    let area = 4.0 * PI * r * r;
    let total_heat_loss = q * area;
    let cooling_rate = total_heat_loss / (m * cp);
    let strain_rate = alpha * cooling_rate;

    if !strain_rate.is_finite() || strain_rate <= 0.0 {
        0.0
    } else {
        strain_rate
    }
}

pub fn tectonic_seismic_energy_rate(
    regime: TectonicRegime,
    plate_velocity: Speed,
    brittle_thickness: Length,
    planet_radius: Length,
    planet_mass: Mass,
    surface_heat_flux: HeatFlux,
    lithosphere_yield_stress: Pressure,
    shear_modulus: Pressure,
    plate_count: u32,
    thermal_expansion: f64,
    specific_heat_capacity: f64,
) -> Luminosity {
    let eta = seismic_efficiency(lithosphere_yield_stress, shear_modulus);
    let sigma_y = lithosphere_yield_stress.value();
    let r = planet_radius.value();
    let z_b = brittle_thickness.value();

    if sigma_y <= 0.0
        || r <= 0.0
        || z_b <= 0.0
        || !sigma_y.is_finite()
        || !r.is_finite()
        || !z_b.is_finite()
    {
        return Luminosity::new(0.0);
    }

    match regime {
        TectonicRegime::PlateTectonics | TectonicRegime::IceTectonics => {
            let v = plate_velocity.value();
            if v <= 0.0 || !v.is_finite() {
                return Luminosity::new(0.0);
            }

            let n_plates = plate_count.max(2) as f64;
            let boundary_length = n_plates.sqrt() * PI * r;
            let fault_area_rate = boundary_length * z_b.min(r) * v;
            let power = sigma_y * fault_area_rate * eta;

            if !power.is_finite() || power <= 0.0 {
                Luminosity::new(0.0)
            } else {
                Luminosity::new(power)
            }
        }
        TectonicRegime::StagnantLid => {
            let strain_rate = thermal_contraction_strain_rate(
                surface_heat_flux,
                planet_mass,
                planet_radius,
                thermal_expansion,
                specific_heat_capacity,
            );

            if strain_rate <= 0.0 {
                return Luminosity::new(0.0);
            }

            let v_brittle = spherical_shell_volume(planet_radius, brittle_thickness, 1.0);
            let power = sigma_y * v_brittle * strain_rate * eta;

            if !power.is_finite() || power <= 0.0 {
                Luminosity::new(0.0)
            } else {
                Luminosity::new(power)
            }
        }
        TectonicRegime::HeatPipe | TectonicRegime::Inactive => Luminosity::new(0.0),
    }
}