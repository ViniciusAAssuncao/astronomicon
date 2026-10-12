use astronomicon_core::math::kepler::{mean_motion, orbital_period};
use astronomicon_core::units::{
    AngularVelocity, Duration, GravitationalParameter, Length,
};
use std::f64::consts::TAU;

pub fn sidereal_year_length(
    semi_major_axis: Length,
    central_mu: GravitationalParameter,
) -> Option<Duration> {
    orbital_period(semi_major_axis, central_mu)
}

pub fn anomalistic_year_length(
    semi_major_axis: Length,
    central_mu: GravitationalParameter,
    apsidal_precession_rate: AngularVelocity,
) -> Option<Duration> {
    let n = mean_motion(semi_major_axis, central_mu).value();
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

pub fn tropical_year_length(
    sidereal_year: Duration,
    axial_precession_rate: AngularVelocity,
) -> Option<Duration> {
    let t_sid = sidereal_year.value();
    let d_psi = axial_precession_rate.value();

    if t_sid <= 0.0 || !t_sid.is_finite() || !d_psi.is_finite() {
        return None;
    }

    if d_psi == 0.0 {
        return Some(sidereal_year);
    }

    let n = TAU / t_sid;
    let n_trop = n + d_psi;

    if n_trop <= 0.0 || !n_trop.is_finite() {
        return None;
    }

    Some(Duration::new(TAU / n_trop))
}