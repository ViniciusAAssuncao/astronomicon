use crate::math::black_hole::energetics::eddington_luminosity;
use crate::units::constants::{GRAVITATIONAL_CONSTANT, SPEED_OF_LIGHT};
use crate::units::{Density, Length, Luminosity, Mass, MassRate, Speed};
use std::f64::consts::PI;

pub fn tidal_disruption_radius(
    black_hole_mass: Mass,
    body_mass: Mass,
    body_radius: Length,
) -> Length {
    let m_bh = black_hole_mass.value();
    let m_body = body_mass.value();
    let r_body = body_radius.value();

    if m_bh <= 0.0
        || m_body <= 0.0
        || r_body <= 0.0
        || !m_bh.is_finite()
        || !m_body.is_finite()
        || !r_body.is_finite()
    {
        return Length::new(0.0);
    }

    let r_t = r_body * (m_bh / m_body).cbrt();
    if !r_t.is_finite() || r_t <= 0.0 {
        Length::new(0.0)
    } else {
        Length::new(r_t)
    }
}

pub fn bondi_hoyle_lyttleton_accretion_rate(
    black_hole_mass: Mass,
    wind_density: Density,
    wind_speed: Speed,
    relative_orbital_speed: Speed,
) -> MassRate {
    let m = black_hole_mass.value();
    let rho = wind_density.value();
    let v_w = wind_speed.value();
    let v_orb = relative_orbital_speed.value();

    if m <= 0.0 || rho <= 0.0 || !m.is_finite() || !rho.is_finite() {
        return MassRate::new(0.0);
    }

    let v_eff_sq = v_w * v_w + v_orb * v_orb;
    let v_eff = v_eff_sq.sqrt();
    if v_eff <= 0.0 || !v_eff.is_finite() {
        return MassRate::new(0.0);
    }

    let g = GRAVITATIONAL_CONSTANT;
    let m_dot = (4.0 * PI * g * g * m * m * rho) / (v_eff * v_eff * v_eff);

    if !m_dot.is_finite() || m_dot < 0.0 {
        MassRate::new(0.0)
    } else {
        MassRate::new(m_dot)
    }
}

pub fn accretion_disk_luminosity(
    accretion_rate: MassRate,
    radiative_efficiency: f64,
    black_hole_mass: Mass,
) -> Luminosity {
    let m_dot = accretion_rate.value();
    let eta = radiative_efficiency.clamp(0.0, 1.0);
    let m = black_hole_mass.value();

    if m_dot <= 0.0 || eta <= 0.0 || m <= 0.0 || !m_dot.is_finite() || !m.is_finite() {
        return Luminosity::new(0.0);
    }

    let c = SPEED_OF_LIGHT;
    let raw_lum = eta * m_dot * c * c;
    let edd_lum = eddington_luminosity(black_hole_mass).value();

    let lum = raw_lum.min(edd_lum);
    if !lum.is_finite() || lum < 0.0 {
        Luminosity::new(0.0)
    } else {
        Luminosity::new(lum)
    }
}
