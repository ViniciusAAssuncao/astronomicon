use crate::domain::PlanetKind;
use crate::units::constants::{GRAVITATIONAL_CONSTANT, ROCHE_FLUID_COEFFICIENT};
use crate::units::{Density, Duration, GravitationalParameter, HeatFlux, Length, Luminosity, Mass};
use std::f64::consts::PI;

pub fn roche_limit_rigid(
    primary_radius: Length,
    primary_density: Density,
    satellite_density: Density,
) -> Length {
    let r_p = primary_radius.value();
    let rho_p = primary_density.value();
    let rho_s = satellite_density.value();

    if r_p <= 0.0
        || rho_p <= 0.0
        || rho_s <= 0.0
        || !r_p.is_finite()
        || !rho_p.is_finite()
        || !rho_s.is_finite()
    {
        return Length::new(0.0);
    }

    let ratio = 2.0 * rho_p / rho_s;
    Length::new(r_p * ratio.cbrt())
}

pub fn roche_limit_fluid(
    primary_radius: Length,
    primary_density: Density,
    satellite_density: Density,
) -> Length {
    let r_p = primary_radius.value();
    let rho_p = primary_density.value();
    let rho_s = satellite_density.value();

    if r_p <= 0.0
        || rho_p <= 0.0
        || rho_s <= 0.0
        || !r_p.is_finite()
        || !rho_p.is_finite()
        || !rho_s.is_finite()
    {
        return Length::new(0.0);
    }

    let ratio = rho_p / rho_s;
    Length::new(ROCHE_FLUID_COEFFICIENT * r_p * ratio.cbrt())
}

pub fn synchronous_orbit_radius(
    mu_primary: GravitationalParameter,
    rotation_period: Duration,
) -> Length {
    let mu = mu_primary.value();
    let t = rotation_period.value();

    if mu <= 0.0 || t <= 0.0 || !mu.is_finite() || !t.is_finite() {
        return Length::new(0.0);
    }

    let val = (mu * t * t) / (4.0 * PI * PI);
    Length::new(val.cbrt())
}

pub fn tidal_heating_total_power(
    parent_mass: Mass,
    satellite_mass: Mass,
    semi_major_axis: Length,
    eccentricity: f64,
    satellite_radius: Length,
    love_number_k2: f64,
    tidal_dissipation_factor_q: f64,
) -> Luminosity {
    let m_p = parent_mass.value();
    let m_s = satellite_mass.value();
    let a = semi_major_axis.value();
    let e = eccentricity;
    let r = satellite_radius.value();
    let k2 = love_number_k2;
    let q = tidal_dissipation_factor_q;

    if m_p <= 0.0
        || m_s < 0.0
        || a <= 0.0
        || e < 0.0
        || e >= 1.0
        || r <= 0.0
        || k2 <= 0.0
        || q <= 0.0
        || !m_p.is_finite()
        || !m_s.is_finite()
        || !a.is_finite()
        || !e.is_finite()
        || !r.is_finite()
        || !k2.is_finite()
        || !q.is_finite()
    {
        return Luminosity::new(0.0);
    }

    let mu = GRAVITATIONAL_CONSTANT * (m_p + m_s);
    let n = (mu / a.powi(3)).sqrt();

    let num = 10.5 * (k2 / q) * GRAVITATIONAL_CONSTANT * m_p * m_p * r.powi(5) * n * e * e;
    let den = a.powi(6);

    let power = num / den;
    if !power.is_finite() || power <= 0.0 {
        Luminosity::new(0.0)
    } else {
        Luminosity::new(power)
    }
}

pub fn tidal_heating_surface_flux(
    parent_mass: Mass,
    satellite_mass: Mass,
    semi_major_axis: Length,
    eccentricity: f64,
    satellite_radius: Length,
    love_number_k2: f64,
    tidal_dissipation_factor_q: f64,
) -> HeatFlux {
    let r = satellite_radius.value();
    if r <= 0.0 || !r.is_finite() {
        return HeatFlux::new(0.0);
    }

    let total_power = tidal_heating_total_power(
        parent_mass,
        satellite_mass,
        semi_major_axis,
        eccentricity,
        satellite_radius,
        love_number_k2,
        tidal_dissipation_factor_q,
    );

    let area = 4.0 * PI * r * r;
    if area <= 0.0 || !area.is_finite() {
        return HeatFlux::new(0.0);
    }

    HeatFlux::new(total_power.value() / area)
}

pub fn fallback_love_number_k2(kind: PlanetKind, mean_density: Option<Density>) -> f64 {
    match kind {
        PlanetKind::Telluric | PlanetKind::Chthonian => {
            if let Some(rho) = mean_density {
                let d = rho.value();
                if d > 5500.0 {
                    0.32
                } else if d > 4000.0 {
                    0.30
                } else if d > 0.0 {
                    0.25
                } else {
                    0.30
                }
            } else {
                0.30
            }
        }
        PlanetKind::CarbonPlanet => 0.28,
        PlanetKind::GasGiant => {
            if let Some(rho) = mean_density {
                let d = rho.value();
                if d > 1500.0 {
                    0.45
                } else if d > 800.0 {
                    0.50
                } else if d > 0.0 {
                    0.55
                } else {
                    0.50
                }
            } else {
                0.50
            }
        }
        PlanetKind::IceGiant => 0.38,
        PlanetKind::IcyBody | PlanetKind::DwarfPlanet => {
            if let Some(rho) = mean_density {
                let d = rho.value();
                if d > 3000.0 {
                    0.20
                } else if d > 2000.0 {
                    0.10
                } else if d > 0.0 {
                    0.05
                } else {
                    0.08
                }
            } else {
                0.08
            }
        }
        PlanetKind::Exotic => 0.30,
    }
}

pub fn fallback_tidal_dissipation_factor_q(kind: PlanetKind) -> f64 {
    match kind {
        PlanetKind::Telluric => 100.0,
        PlanetKind::Chthonian => 100.0,
        PlanetKind::CarbonPlanet => 100.0,
        PlanetKind::GasGiant => 50000.0,
        PlanetKind::IceGiant => 10000.0,
        PlanetKind::IcyBody => 35.0,
        PlanetKind::DwarfPlanet => 50.0,
        PlanetKind::Exotic => 100.0,
    }
}

pub fn tidal_locking_timescale(
    satellite_mass: Mass,
    satellite_radius: Length,
    initial_rotation_period: Duration,
    semi_major_axis: Length,
    parent_mass: Mass,
    love_number_k2: f64,
    tidal_dissipation_factor_q: f64,
) -> Duration {
    let m_s = satellite_mass.value();
    let r_s = satellite_radius.value();
    let t_rot = initial_rotation_period.value();
    let a = semi_major_axis.value();
    let m_p = parent_mass.value();
    let k2 = love_number_k2;
    let q = tidal_dissipation_factor_q;

    if m_s <= 0.0
        || r_s <= 0.0
        || t_rot <= 0.0
        || a <= 0.0
        || m_p <= 0.0
        || k2 <= 0.0
        || q <= 0.0
        || !m_s.is_finite()
        || !r_s.is_finite()
        || !t_rot.is_finite()
        || !a.is_finite()
        || !m_p.is_finite()
        || !k2.is_finite()
        || !q.is_finite()
    {
        return Duration::new(0.0);
    }

    let omega_0 = (2.0 * PI) / t_rot;
    let num = 2.0 * omega_0 * a.powi(6) * m_s * q;
    let den = 15.0 * GRAVITATIONAL_CONSTANT * m_p * m_p * k2 * r_s.powi(3);

    if den <= 0.0 || !den.is_finite() {
        return Duration::new(0.0);
    }

    let t_lock = num / den;
    if !t_lock.is_finite() || t_lock <= 0.0 {
        Duration::new(0.0)
    } else {
        Duration::new(t_lock)
    }
}
