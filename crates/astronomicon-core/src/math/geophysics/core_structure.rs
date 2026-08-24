use crate::domain::PlanetKind;
use crate::units::constants::EARTH_MASS;
use crate::units::{Density, Length, Mass};
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
