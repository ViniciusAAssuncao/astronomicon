use crate::units::constants::{
    SECONDS_PER_YEAR, SOLAR_LUMINOSITY, SOLAR_MASS, SOLAR_RADIUS,
};
use crate::units::{Density, Length, Luminosity, Mass, MassRate, Pressure, Speed};
use std::f64::consts::PI;

pub fn reimers_mass_loss_rate(
    luminosity: Luminosity,
    radius: Length,
    mass: Mass,
    eta: f64,
) -> MassRate {
    let l = luminosity.value();
    let r = radius.value();
    let m = mass.value();

    if l <= 0.0 || r <= 0.0 || m <= 0.0 || eta <= 0.0 || !l.is_finite() || !r.is_finite() || !m.is_finite() || !eta.is_finite() {
        return MassRate::new(0.0);
    }

    let l_sol = l / SOLAR_LUMINOSITY;
    let r_sol = r / SOLAR_RADIUS;
    let m_sol = m / SOLAR_MASS;

    let m_dot_solar_per_yr = 4.0e-13 * eta * ((l_sol * r_sol) / m_sol);
    let m_dot_kg_per_s = m_dot_solar_per_yr * (SOLAR_MASS / SECONDS_PER_YEAR);

    MassRate::new(m_dot_kg_per_s)
}

pub fn terminal_wind_speed(escape_velocity: Speed, scaling_factor: f64) -> Speed {
    let v_esc = escape_velocity.value();
    if v_esc <= 0.0 || scaling_factor <= 0.0 || !v_esc.is_finite() || !scaling_factor.is_finite() {
        return Speed::new(0.0);
    }
    Speed::new(v_esc * scaling_factor)
}

pub fn stellar_wind_density(
    mass_loss_rate: MassRate,
    terminal_speed: Speed,
    distance: Length,
) -> Density {
    let m_dot = mass_loss_rate.value();
    let v_inf = terminal_speed.value();
    let r = distance.value();

    if m_dot <= 0.0 || v_inf <= 0.0 || r <= 0.0 || !m_dot.is_finite() || !v_inf.is_finite() || !r.is_finite() {
        return Density::new(0.0);
    }

    let area_flux = 4.0 * PI * r * r * v_inf;
    Density::new(m_dot / area_flux)
}

pub fn stellar_wind_dynamic_pressure(wind_density: Density, terminal_speed: Speed) -> Pressure {
    let rho = wind_density.value();
    let v_inf = terminal_speed.value();

    if rho <= 0.0 || v_inf <= 0.0 || !rho.is_finite() || !v_inf.is_finite() {
        return Pressure::new(0.0);
    }

    Pressure::new(rho * v_inf * v_inf)
}