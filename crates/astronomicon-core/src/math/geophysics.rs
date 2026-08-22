use crate::domain::PlanetKind;
use crate::units::constants::{
    CORE_ADIABATIC_HEAT_FLUX_REF, CORE_COOLING_TIMESCALE_BASE_YEARS, CORE_GRAVITY_EARTH_REF,
    CORE_PRIMORDIAL_HEAT_FLUX_REF, CORE_RADIOGENIC_DECAY_TIMESCALE_YEARS,
    CORE_RADIOGENIC_HEAT_FLUX_REF, EARTH_MASS, GRAVITATIONAL_CONSTANT, SECONDS_PER_YEAR,
};
use crate::units::{Density, Duration, HeatFlux, Length, Mass};
use std::f64::consts::PI;

pub fn conducting_core_radius(
    planet_radius: Length,
    planet_mass: Mass,
    kind: PlanetKind,
    core_mass_fraction: f64,
) -> Length {
    let r_p = planet_radius.value();
    let m_p = planet_mass.value();

    if r_p <= 0.0
        || m_p <= 0.0
        || !r_p.is_finite()
        || !m_p.is_finite()
        || !core_mass_fraction.is_finite()
        || core_mass_fraction <= 0.0
    {
        return Length::new(0.0);
    }

    let cmf = core_mass_fraction.clamp(0.0, 1.0);
    if cmf >= 1.0 {
        return planet_radius;
    }

    let base_density_ratio = match kind {
        PlanetKind::Telluric => 1.9,
        PlanetKind::Chthonian => 1.95,
        PlanetKind::CarbonPlanet => 1.65,
        PlanetKind::DwarfPlanet => 1.45,
        PlanetKind::IcyBody => 1.4,
        PlanetKind::GasGiant => 1.3,
        PlanetKind::IceGiant => 1.25,
        PlanetKind::Exotic => 1.5,
    };

    let mass_ratio = m_p / EARTH_MASS;
    let compression = 1.0 + 0.08 * (1.0 + mass_ratio).ln().max(0.0);
    let density_ratio = base_density_ratio * compression;

    let r_c = r_p * (cmf / density_ratio).powf(1.0 / 3.0);
    Length::new(r_c.clamp(0.0, r_p))
}

pub fn core_density(planet_mass: Mass, core_mass_fraction: f64, core_radius: Length) -> Density {
    let m_p = planet_mass.value();
    let r_c = core_radius.value();

    if m_p <= 0.0
        || r_c <= 0.0
        || !m_p.is_finite()
        || !r_c.is_finite()
        || !core_mass_fraction.is_finite()
        || core_mass_fraction <= 0.0
    {
        return Density::new(0.0);
    }

    let cmf = core_mass_fraction.clamp(0.0, 1.0);
    let core_m = m_p * cmf;
    let volume = (4.0 / 3.0) * PI * r_c.powi(3);

    if volume <= 0.0 || !volume.is_finite() {
        return Density::new(0.0);
    }

    Density::new(core_m / volume)
}

pub fn core_mantle_boundary_heat_flux(
    planet_mass: Mass,
    core_radius: Length,
    core_mass_fraction: f64,
    radioactive_heating_rate: f64,
    age: Duration,
) -> HeatFlux {
    let m_p = planet_mass.value();
    let r_c = core_radius.value();

    if m_p <= 0.0
        || r_c <= 0.0
        || !m_p.is_finite()
        || !r_c.is_finite()
        || !core_mass_fraction.is_finite()
        || core_mass_fraction <= 0.0
    {
        return HeatFlux::new(0.0);
    }

    let mass_ratio = m_p / EARTH_MASS;
    let t_sec = age.value().max(0.0);

    let tau_cool = CORE_COOLING_TIMESCALE_BASE_YEARS * SECONDS_PER_YEAR * mass_ratio.powf(1.0 / 3.0).max(0.1);
    let q_prim = CORE_PRIMORDIAL_HEAT_FLUX_REF * mass_ratio.powf(2.0 / 3.0) * (-t_sec / tau_cool).exp();

    let r_rate = if radioactive_heating_rate.is_finite() && radioactive_heating_rate > 0.0 {
        radioactive_heating_rate
    } else {
        0.0
    };
    let tau_rad = CORE_RADIOGENIC_DECAY_TIMESCALE_YEARS * SECONDS_PER_YEAR;
    let q_rad = CORE_RADIOGENIC_HEAT_FLUX_REF * r_rate * mass_ratio.powf(1.0 / 3.0) * (-t_sec / tau_rad).exp();

    let total_q = q_prim + q_rad;
    if !total_q.is_finite() || total_q <= 0.0 {
        HeatFlux::new(0.0)
    } else {
        HeatFlux::new(total_q)
    }
}

pub fn radiogenic_heat_flux(
    planet_mass: Mass,
    radioactive_heating_rate: f64,
    age: Duration,
) -> HeatFlux {
    let m_p = planet_mass.value();
    if m_p <= 0.0 || !m_p.is_finite() {
        return HeatFlux::new(0.0);
    }

    let mass_ratio = m_p / EARTH_MASS;
    let t_sec = age.value().max(0.0);

    let r_rate = if radioactive_heating_rate.is_finite() && radioactive_heating_rate > 0.0 {
        radioactive_heating_rate
    } else {
        0.0
    };
    let tau_rad = CORE_RADIOGENIC_DECAY_TIMESCALE_YEARS * SECONDS_PER_YEAR;
    let q_rad = CORE_RADIOGENIC_HEAT_FLUX_REF * r_rate * mass_ratio.powf(1.0 / 3.0) * (-t_sec / tau_rad).exp();

    if !q_rad.is_finite() || q_rad <= 0.0 {
        HeatFlux::new(0.0)
    } else {
        HeatFlux::new(q_rad)
    }
}

pub fn core_adiabatic_heat_flux(core_mass: Mass, core_radius: Length) -> HeatFlux {
    let m_c = core_mass.value();
    let r_c = core_radius.value();

    if m_c <= 0.0 || r_c <= 0.0 || !m_c.is_finite() || !r_c.is_finite() {
        return HeatFlux::new(0.0);
    }

    let g_c = (GRAVITATIONAL_CONSTANT * m_c) / (r_c * r_c);
    if !g_c.is_finite() || g_c <= 0.0 {
        return HeatFlux::new(0.0);
    }

    let q_ad = CORE_ADIABATIC_HEAT_FLUX_REF * (g_c / CORE_GRAVITY_EARTH_REF);

    if !q_ad.is_finite() || q_ad <= 0.0 {
        HeatFlux::new(0.0)
    } else {
        HeatFlux::new(q_ad)
    }
}

pub fn convective_core_heat_flux(
    planet_mass: Mass,
    core_radius: Length,
    core_mass_fraction: f64,
    radioactive_heating_rate: f64,
    age: Duration,
) -> HeatFlux {
    let q_cmb = core_mantle_boundary_heat_flux(
        planet_mass,
        core_radius,
        core_mass_fraction,
        radioactive_heating_rate,
        age,
    );

    let cmf = core_mass_fraction.clamp(0.0, 1.0);
    let m_c = Mass::new(planet_mass.value() * cmf);
    let q_ad = core_adiabatic_heat_flux(m_c, core_radius);

    let q_conv = q_cmb.value() - q_ad.value();
    if !q_conv.is_finite() || q_conv <= 0.0 {
        HeatFlux::new(0.0)
    } else {
        HeatFlux::new(q_conv)
    }
}

pub fn total_surface_geothermal_heat_flux(
    internal_heat_flux: HeatFlux,
    tidal_heat_flux: HeatFlux,
) -> HeatFlux {
    let q_int = if internal_heat_flux.value().is_finite() && internal_heat_flux.value() > 0.0 {
        internal_heat_flux.value()
    } else {
        0.0
    };
    let q_tide = if tidal_heat_flux.value().is_finite() && tidal_heat_flux.value() > 0.0 {
        tidal_heat_flux.value()
    } else {
        0.0
    };

    HeatFlux::new(q_int + q_tide)
}
