use crate::domain::orbital_elements::OrbitalElements;
use crate::error::{ DomainError, DomainResult };
use crate::math::gravity::combined_gravitational_parameter;
use crate::math::perturbation::SecularPrecessionRates;
use crate::units::{
    Angle,
    AngularVelocity,
    Duration,
    GravitationalParameter,
    Length,
    Mass,
    Position,
    Speed,
    Vector3,
    Velocity,
};
use std::f64::consts::{ PI, TAU };

pub fn orbital_period(semi_major_axis: Length, mu: GravitationalParameter) -> Option<Duration> {
    if semi_major_axis.value() <= 0.0 || mu.value() <= 0.0 {
        None
    } else {
        Some(Duration::new(2.0 * PI * (semi_major_axis.value().powi(3) / mu.value()).sqrt()))
    }
}

pub fn mean_motion(semi_major_axis: Length, mu: GravitationalParameter) -> AngularVelocity {
    if semi_major_axis.value() <= 0.0 || mu.value() <= 0.0 {
        AngularVelocity::new(0.0)
    } else {
        AngularVelocity::new((mu.value() / semi_major_axis.value().abs().powi(3)).sqrt())
    }
}

pub fn orbital_speed(mu: GravitationalParameter, radius: Length, semi_major_axis: Length) -> Speed {
    if radius.value() <= 0.0 || semi_major_axis.value() <= 0.0 || mu.value() <= 0.0 {
        Speed::new(0.0)
    } else {
        let v_sq = mu.value() * (2.0 / radius.value() - 1.0 / semi_major_axis.value());
        Speed::new(v_sq.max(0.0).sqrt())
    }
}

pub fn star_barycentric_mu(star_mass: Mass, other_stars_mass: Mass) -> GravitationalParameter {
    combined_gravitational_parameter(star_mass, other_stars_mass)
}

pub fn body_orbit_mu(body_mass: Mass, parent_mass: Mass) -> GravitationalParameter {
    combined_gravitational_parameter(body_mass, parent_mass)
}

pub fn solve_kepler(mean_anomaly: Angle, eccentricity: f64) -> DomainResult<Angle> {
    let m = mean_anomaly.value().rem_euclid(TAU);
    let mut e_anom = m;
    let max_iter = 100;
    let tolerance = 1e-10;

    for _ in 0..max_iter {
        let f = e_anom - eccentricity * e_anom.sin() - m;
        let f_prime = 1.0 - eccentricity * e_anom.cos();
        let delta = f / f_prime;
        e_anom -= delta;
        if delta.abs() < tolerance {
            return Ok(Angle::new(e_anom));
        }
    }

    Err(DomainError::NumericalConvergence {
        context: "kepler_solver".to_string(),
        reason: "failed to converge within maximum iterations".to_string(),
    })
}

pub fn true_anomaly_from_eccentric(eccentric_anomaly: Angle, eccentricity: f64) -> Angle {
    let e_anom = eccentric_anomaly.value();
    let sin_nu = (1.0 - eccentricity * eccentricity).max(0.0).sqrt() * e_anom.sin();
    let cos_nu = e_anom.cos() - eccentricity;
    Angle::new(sin_nu.atan2(cos_nu))
}

pub fn perifocal_state_vectors(
    semi_major_axis: Length,
    eccentricity: f64,
    true_anomaly: Angle,
    mu: GravitationalParameter
) -> (Position, Velocity) {
    let a = semi_major_axis.value();
    let e = eccentricity;
    let nu = true_anomaly.value();
    let p = a * (1.0 - e * e);
    let r = p / (1.0 + e * nu.cos());

    let r_pqw = Vector3::new(r * nu.cos(), r * nu.sin(), 0.0);

    let h = (mu.value() * p).sqrt();
    let v_factor = if h > 0.0 { mu.value() / h } else { 0.0 };
    let v_pqw = Vector3::new(-v_factor * nu.sin(), v_factor * (e + nu.cos()), 0.0);

    (Position::from_raw(r_pqw), Velocity::from_raw(v_pqw))
}

pub fn rotate_perifocal_to_system(
    position: Position,
    velocity: Velocity,
    argument_of_periapsis: Angle,
    inclination: Angle,
    longitude_of_ascending_node: Angle
) -> (Position, Velocity) {
    let omega = argument_of_periapsis.value();
    let inc = inclination.value();
    let raan = longitude_of_ascending_node.value();

    let r = position.raw().rotate_about_z(omega).rotate_about_x(inc).rotate_about_z(raan);

    let v = velocity.raw().rotate_about_z(omega).rotate_about_x(inc).rotate_about_z(raan);

    (Position::from_raw(r), Velocity::from_raw(v))
}

pub fn propagate_mean_anomaly(
    mean_anomaly_at_epoch: Angle,
    mean_motion: AngularVelocity,
    time_since_epoch: Duration
) -> Angle {
    Angle::new((mean_anomaly_at_epoch + mean_motion * time_since_epoch).value().rem_euclid(TAU))
}

pub fn mean_longitude_at_epoch(
    elements: &OrbitalElements,
    mean_motion: AngularVelocity,
    time_since_epoch: Duration
) -> Angle {
    let mean_anomaly = propagate_mean_anomaly(
        elements.mean_anomaly_at_epoch(),
        mean_motion,
        time_since_epoch
    );
    Angle::new((elements.longitude_of_periapsis().value() + mean_anomaly.value()).rem_euclid(TAU))
}

pub fn true_anomaly_at_epoch(
    elements: &OrbitalElements,
    mu: GravitationalParameter,
    time_since_epoch: Duration
) -> DomainResult<Angle> {
    let n = mean_motion(elements.semi_major_axis(), mu);
    let mean_anomaly = propagate_mean_anomaly(
        elements.mean_anomaly_at_epoch(),
        n,
        time_since_epoch
    );
    let eccentric_anomaly = solve_kepler(mean_anomaly, elements.eccentricity())?;
    Ok(true_anomaly_from_eccentric(eccentric_anomaly, elements.eccentricity()))
}

pub fn true_anomaly_at_epoch_secular(
    elements: &OrbitalElements,
    mu: GravitationalParameter,
    secular_rates: &SecularPrecessionRates,
    time_since_epoch: Duration
) -> DomainResult<Angle> {
    let n = mean_motion(elements.semi_major_axis(), mu);
    let n_corr = n + secular_rates.mean_anomaly_correction;
    let mean_anomaly = propagate_mean_anomaly(
        elements.mean_anomaly_at_epoch(),
        n_corr,
        time_since_epoch
    );
    let eccentric_anomaly = solve_kepler(mean_anomaly, elements.eccentricity())?;
    Ok(true_anomaly_from_eccentric(eccentric_anomaly, elements.eccentricity()))
}

pub fn orbital_state_vectors(
    elements: &OrbitalElements,
    mu: GravitationalParameter,
    time_since_epoch: Duration
) -> DomainResult<(Position, Velocity)> {
    let true_anom = true_anomaly_at_epoch(elements, mu, time_since_epoch)?;

    let (r_pqw, v_pqw) = perifocal_state_vectors(
        elements.semi_major_axis(),
        elements.eccentricity(),
        true_anom,
        mu
    );

    Ok(
        rotate_perifocal_to_system(
            r_pqw,
            v_pqw,
            elements.argument_of_periapsis(),
            elements.inclination(),
            elements.longitude_of_ascending_node()
        )
    )
}

pub fn orbital_state_vectors_secular(
    elements: &OrbitalElements,
    mu: GravitationalParameter,
    secular_rates: &SecularPrecessionRates,
    time_since_epoch: Duration
) -> DomainResult<(Position, Velocity)> {
    let true_anom = true_anomaly_at_epoch_secular(elements, mu, secular_rates, time_since_epoch)?;

    let (r_pqw, v_pqw) = perifocal_state_vectors(
        elements.semi_major_axis(),
        elements.eccentricity(),
        true_anom,
        mu
    );

    let omega_t = Angle::new(
        (elements.argument_of_periapsis() + secular_rates.apsidal * time_since_epoch)
            .value()
            .rem_euclid(TAU)
    );
    let raan_t = Angle::new(
        (elements.longitude_of_ascending_node() + secular_rates.nodal * time_since_epoch)
            .value()
            .rem_euclid(TAU)
    );

    Ok(rotate_perifocal_to_system(r_pqw, v_pqw, omega_t, elements.inclination(), raan_t))
}

pub fn orbital_position(
    elements: &OrbitalElements,
    mu: GravitationalParameter,
    time_since_epoch: Duration
) -> DomainResult<Position> {
    orbital_state_vectors(elements, mu, time_since_epoch).map(|(r, _)| r)
}

pub fn orbital_position_secular(
    elements: &OrbitalElements,
    mu: GravitationalParameter,
    secular_rates: &SecularPrecessionRates,
    time_since_epoch: Duration
) -> DomainResult<Position> {
    orbital_state_vectors_secular(elements, mu, secular_rates, time_since_epoch).map(|(r, _)| r)
}
