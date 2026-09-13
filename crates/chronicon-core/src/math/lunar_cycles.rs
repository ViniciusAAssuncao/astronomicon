use crate::math::synodic::synodic_beat_period;
use astronomicon_core::math::gravity::combined_gravitational_parameter;
use astronomicon_core::math::kepler::{mean_motion, orbital_period};
use astronomicon_core::math::perturbation::{
    apsidal_precession_rate_j2, nodal_regression_rate_j2,
};
use astronomicon_core::units::{
    Angle, AngularVelocity, Duration, Length, Mass,
};
use serde::{Deserialize, Serialize};
use std::f64::consts::{FRAC_PI_2, TAU};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LunarMonths {
    pub sidereal: Option<Duration>,
    pub synodic: Option<Duration>,
    pub anomalistic: Option<Duration>,
    pub draconic: Option<Duration>,
    pub apsidal_precession_rate: AngularVelocity,
    pub nodal_precession_rate: AngularVelocity,
    pub is_retrograde_orbit: bool,
}

impl LunarMonths {
    pub fn new(
        sidereal: Option<Duration>,
        synodic: Option<Duration>,
        anomalistic: Option<Duration>,
        draconic: Option<Duration>,
        apsidal_precession_rate: AngularVelocity,
        nodal_precession_rate: AngularVelocity,
        is_retrograde_orbit: bool,
    ) -> Self {
        Self {
            sidereal,
            synodic,
            anomalistic,
            draconic,
            apsidal_precession_rate,
            nodal_precession_rate,
            is_retrograde_orbit,
        }
    }
}

pub fn is_retrograde_inclination(inclination: Angle) -> bool {
    let val = inclination.value();
    val > FRAC_PI_2 && val.is_finite()
}

pub fn lunar_sidereal_month(
    semi_major_axis: Length,
    host_planet_mass: Mass,
    moon_mass: Mass,
) -> Option<Duration> {
    let mu = combined_gravitational_parameter(host_planet_mass, moon_mass);
    orbital_period(semi_major_axis, mu)
}

pub fn lunar_synodic_month(
    lunar_sidereal_period: Duration,
    planet_orbital_period: Duration,
    is_retrograde_orbit: bool,
) -> Option<Duration> {
    synodic_beat_period(lunar_sidereal_period, planet_orbital_period, !is_retrograde_orbit)
}

pub fn lunar_anomalistic_month(
    semi_major_axis: Length,
    host_planet_mass: Mass,
    moon_mass: Mass,
    apsidal_precession_rate: AngularVelocity,
) -> Option<Duration> {
    let mu = combined_gravitational_parameter(host_planet_mass, moon_mass);
    let n = mean_motion(semi_major_axis, mu).value();
    let d_varpi = apsidal_precession_rate.value();

    if n <= 0.0 || !n.is_finite() || !d_varpi.is_finite() {
        return None;
    }

    let n_anom = n - d_varpi;
    if n_anom <= 0.0 || !n_anom.is_finite() {
        return None;
    }

    Some(Duration::new(TAU / n_anom))
}

pub fn lunar_draconic_month(
    semi_major_axis: Length,
    host_planet_mass: Mass,
    moon_mass: Mass,
    nodal_precession_rate: AngularVelocity,
) -> Option<Duration> {
    let mu = combined_gravitational_parameter(host_planet_mass, moon_mass);
    let n = mean_motion(semi_major_axis, mu).value();
    let d_node = nodal_precession_rate.value();

    if n <= 0.0 || !n.is_finite() || !d_node.is_finite() {
        return None;
    }

    let n_drac = n - d_node;
    if n_drac <= 0.0 || !n_drac.is_finite() {
        return None;
    }

    Some(Duration::new(TAU / n_drac))
}

pub fn lunar_nodal_month(
    semi_major_axis: Length,
    host_planet_mass: Mass,
    moon_mass: Mass,
    nodal_precession_rate: AngularVelocity,
) -> Option<Duration> {
    lunar_draconic_month(
        semi_major_axis,
        host_planet_mass,
        moon_mass,
        nodal_precession_rate,
    )
}

pub fn lunar_apsidal_precession_rate_j2(
    mean_motion: AngularVelocity,
    semi_major_axis: Length,
    eccentricity: f64,
    inclination: Angle,
    host_planet_j2: f64,
    host_planet_equatorial_radius: Length,
) -> AngularVelocity {
    apsidal_precession_rate_j2(
        mean_motion,
        semi_major_axis,
        eccentricity,
        inclination,
        host_planet_j2,
        host_planet_equatorial_radius,
    )
}

pub fn lunar_nodal_regression_rate_j2(
    mean_motion: AngularVelocity,
    semi_major_axis: Length,
    eccentricity: f64,
    inclination: Angle,
    host_planet_j2: f64,
    host_planet_equatorial_radius: Length,
) -> AngularVelocity {
    nodal_regression_rate_j2(
        mean_motion,
        semi_major_axis,
        eccentricity,
        inclination,
        host_planet_j2,
        host_planet_equatorial_radius,
    )
}

pub fn calculate_lunar_months(
    semi_major_axis: Length,
    eccentricity: f64,
    inclination: Angle,
    moon_mass: Mass,
    host_planet_mass: Mass,
    host_planet_j2: Option<f64>,
    host_planet_equatorial_radius: Option<Length>,
    planet_orbital_period: Option<Duration>,
) -> LunarMonths {
    let mu = combined_gravitational_parameter(host_planet_mass, moon_mass);
    let t_sid = orbital_period(semi_major_axis, mu);
    let is_retro = is_retrograde_inclination(inclination);

    let t_syn = match (t_sid, planet_orbital_period) {
        (Some(sid), Some(yr)) => lunar_synodic_month(sid, yr, is_retro),
        _ => None,
    };

    let n = mean_motion(semi_major_axis, mu);

    let (rate_apsidal, rate_nodal) = match (host_planet_j2, host_planet_equatorial_radius) {
        (Some(j2), Some(r_eq)) if j2 > 0.0 && r_eq.value() > 0.0 => {
            let aps = lunar_apsidal_precession_rate_j2(
                n,
                semi_major_axis,
                eccentricity,
                inclination,
                j2,
                r_eq,
            );
            let nod = lunar_nodal_regression_rate_j2(
                n,
                semi_major_axis,
                eccentricity,
                inclination,
                j2,
                r_eq,
            );
            (aps, nod)
        }
        _ => (AngularVelocity::new(0.0), AngularVelocity::new(0.0)),
    };

    let t_anom = lunar_anomalistic_month(semi_major_axis, host_planet_mass, moon_mass, rate_apsidal)
        .or(t_sid);

    let t_drac = lunar_draconic_month(semi_major_axis, host_planet_mass, moon_mass, rate_nodal)
        .or(t_sid);

    LunarMonths::new(
        t_sid,
        t_syn,
        t_anom,
        t_drac,
        rate_apsidal,
        rate_nodal,
        is_retro,
    )
}
