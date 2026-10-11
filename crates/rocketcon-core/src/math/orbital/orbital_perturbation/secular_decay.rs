use super::bessel::{ bessel_i0_scaled, bessel_i1_scaled, bessel_i2_scaled, bessel_i3_scaled };
use super::harmonics::{
    secular_apsidal_precession_rate,
    secular_mean_motion_j2_correction,
    secular_nodal_precession_rate,
};
use super::types::{ SecularDecayRates, SecularOrbitDecayPrediction, ZonalHarmonics };
use crate::constants::{
    SECULAR_DECAY_EPSILON,
    SECULAR_DECAY_MIN_ECCENTRICITY,
    SECULAR_DECAY_MIN_PERIAPSIS_BUFFER_M,
    SECULAR_PROPAGATION_MAX_SUBSTEPS,
};
use crate::math::orbital::conversions::osculating_elements_to_cartesian;
use crate::math::orbital::types::{ OrbitType, OsculatingElements };
use astronomicon_core::math::kepler::{ mean_motion, solve_kepler };
use astronomicon_core::units::{
    Angle,
    Density,
    Duration,
    GravitationalParameter,
    Length,
    Position,
    Speed,
    Vector3,
    VelocityVector,
};
use std::f64::consts::TAU;

pub fn mean_anomaly_from_true_anomaly(true_anomaly: Angle, eccentricity: f64) -> Angle {
    let e = eccentricity.clamp(0.0, 0.999);
    let nu = true_anomaly.value();
    let cos_e = (e + nu.cos()) / (1.0 + e * nu.cos());
    let sin_e = ((1.0 - e * e).max(0.0).sqrt() * nu.sin()) / (1.0 + e * nu.cos());
    let e_anom = sin_e.atan2(cos_e);
    let m = (e_anom - e * e_anom.sin()).rem_euclid(TAU);
    Angle::new(m)
}

pub fn secular_drag_decay_rates(
    elements: &OsculatingElements,
    atmospheric_density_periapsis: Density,
    atmospheric_scale_height: Length,
    ballistic_coefficient_inverse: f64,
    mu: GravitationalParameter
) -> (Speed, f64) {
    let a = elements.semi_major_axis.value();
    let e = elements.eccentricity.clamp(0.0, 0.999);
    let rho_p = atmospheric_density_periapsis.value();
    let h_scale = atmospheric_scale_height.value();
    let beta = ballistic_coefficient_inverse;
    let mu_val = mu.value();

    if a <= 0.0 || rho_p <= 0.0 || h_scale <= 0.0 || beta <= 0.0 || mu_val <= 0.0 {
        return (Speed::new(0.0), 0.0);
    }

    let c = ((a * e) / h_scale).max(0.0);
    let sqrt_mu_a = (mu_val * a).sqrt();

    if c < SECULAR_DECAY_MIN_ECCENTRICITY || e < SECULAR_DECAY_MIN_ECCENTRICITY {
        let da_dt = -beta * sqrt_mu_a * rho_p;
        (Speed::new(da_dt), 0.0)
    } else {
        let i0 = bessel_i0_scaled(c);
        let i1 = bessel_i1_scaled(c);
        let i2 = bessel_i2_scaled(c);
        let i3 = bessel_i3_scaled(c);

        let da_factor = i0 + 2.0 * e * i1 + 0.75 * e * e * (i0 + i2);
        let de_factor = i1 + 0.5 * e * (i0 + i2) + 0.125 * e * e * (3.0 * i1 + i3);

        let da_dt = -beta * sqrt_mu_a * rho_p * da_factor;
        let de_dt = -beta * (mu_val / a).sqrt() * rho_p * de_factor;

        (Speed::new(da_dt), de_dt)
    }
}

pub fn secular_orbital_rates(
    elements: &OsculatingElements,
    atmospheric_density_periapsis: Density,
    atmospheric_scale_height: Length,
    ballistic_coefficient_inverse: f64,
    mu: GravitationalParameter,
    eq_radius: Length,
    harmonics: &ZonalHarmonics
) -> SecularDecayRates {
    let (da_dt, de_dt) = secular_drag_decay_rates(
        elements,
        atmospheric_density_periapsis,
        atmospheric_scale_height,
        ballistic_coefficient_inverse,
        mu
    );

    let d_raan_dt = secular_nodal_precession_rate(elements, mu, eq_radius, harmonics);
    let d_w_dt = secular_apsidal_precession_rate(elements, mu, eq_radius, harmonics);
    let d_m_j2 = secular_mean_motion_j2_correction(elements, mu, eq_radius, harmonics.j2);

    SecularDecayRates::new(da_dt, de_dt, d_raan_dt, d_w_dt, d_m_j2)
}

pub fn density_at_periapsis(
    semi_major_axis: Length,
    eccentricity: f64,
    planet_radius: Length,
    atmospheric_surface_density: Density,
    atmospheric_scale_height: Length,
    atmospheric_boundary_altitude: Length
) -> Density {
    let rp = semi_major_axis.value() * (1.0 - eccentricity.clamp(0.0, 0.999));
    let alt = rp - planet_radius.value();
    let h_scale = atmospheric_scale_height.value();
    let rho0 = atmospheric_surface_density.value();
    let h_top = atmospheric_boundary_altitude.value();

    if alt < 0.0 || alt >= h_top || h_scale <= 0.0 || rho0 <= 0.0 {
        Density::new(0.0)
    } else {
        let expo = -alt / h_scale;
        if expo < -700.0 {
            Density::new(0.0)
        } else {
            Density::new(rho0 * expo.exp())
        }
    }
}

pub fn propagate_secular_orbit_decay(
    elements: &OsculatingElements,
    atmospheric_surface_density: Density,
    atmospheric_scale_height: Length,
    atmospheric_boundary_altitude: Length,
    ballistic_coefficient_inverse: f64,
    planet_radius: Length,
    mu: GravitationalParameter,
    harmonics: &ZonalHarmonics,
    duration: Duration,
    max_substeps: usize
) -> SecularOrbitDecayPrediction {
    let total_t = duration.value();
    let n_steps = max_substeps.clamp(1, SECULAR_PROPAGATION_MAX_SUBSTEPS);
    let dt = total_t / (n_steps as f64);

    let mut a = elements.semi_major_axis.value();
    let mut e = elements.eccentricity.clamp(0.0, 0.999);
    let inc = elements.inclination;
    let mut raan = elements.longitude_of_ascending_node.value();
    let mut omega = elements.argument_of_periapsis.value();

    let mut mean_anom = mean_anomaly_from_true_anomaly(elements.true_anomaly, e).value();
    let r_planet = planet_radius.value();

    let mut elapsed = 0.0;
    let mut is_deorbited = false;
    let mut deorbit_offset: Option<Duration> = None;

    for _ in 0..n_steps {
        let rp = a * (1.0 - e);
        let alt_p = rp - r_planet;

        if alt_p <= SECULAR_DECAY_MIN_PERIAPSIS_BUFFER_M {
            is_deorbited = true;
            deorbit_offset = Some(Duration::new(elapsed));
            break;
        }

        let rho_p = density_at_periapsis(
            Length::new(a),
            e,
            planet_radius,
            atmospheric_surface_density,
            atmospheric_scale_height,
            atmospheric_boundary_altitude
        );

        let curr_elem = OsculatingElements::new(
            Length::new(a),
            e,
            inc,
            Angle::new(raan),
            Angle::new(omega),
            Angle::new(0.0),
            Length::new(a * (1.0 - e)),
            Some(Length::new(a * (1.0 + e))),
            0.0,
            Vector3::zero(),
            OrbitType::Elliptic
        );

        let rates = secular_orbital_rates(
            &curr_elem,
            rho_p,
            atmospheric_scale_height,
            ballistic_coefficient_inverse,
            mu,
            planet_radius,
            harmonics
        );

        let da = rates.semi_major_axis_rate.value() * dt;
        let de = rates.eccentricity_rate * dt;
        let draan = rates.nodal_precession_rate.value() * dt;
        let domega = rates.apsidal_precession_rate.value() * dt;

        let n = mean_motion(Length::new(a), mu).value();
        let dm = (n + rates.mean_motion_correction.value()) * dt;

        a = (a + da).max(r_planet);
        e = (e + de).clamp(0.0, 0.999);
        raan = (raan + draan).rem_euclid(TAU);
        omega = (omega + domega).rem_euclid(TAU);
        mean_anom = (mean_anom + dm).rem_euclid(TAU);

        elapsed += dt;

        if a * (1.0 - e) - r_planet <= SECULAR_DECAY_MIN_PERIAPSIS_BUFFER_M {
            is_deorbited = true;
            deorbit_offset = Some(Duration::new(elapsed));
            break;
        }
    }

    let nu = if let Ok(e_anom) = solve_kepler(Angle::new(mean_anom), e) {
        let factor = ((1.0 + e) / (1.0 - e).max(1e-12)).sqrt();
        Angle::new(2.0 * (factor * (0.5 * e_anom.value()).tan()).atan().rem_euclid(TAU))
    } else {
        Angle::new(mean_anom)
    };

    let p_dist = Length::new(a * (1.0 - e));
    let a_dist = if e < 1.0 { Some(Length::new(a * (1.0 + e))) } else { None };
    let orb_type = if e < 1e-6 { OrbitType::Circular } else { OrbitType::Elliptic };

    let final_elements = OsculatingElements::new(
        Length::new(a),
        e,
        inc,
        Angle::new(raan),
        Angle::new(omega),
        nu,
        p_dist,
        a_dist,
        -mu.value() / (2.0 * a),
        Vector3::zero(),
        orb_type
    );

    let (pos, vel) = osculating_elements_to_cartesian(&final_elements, mu).unwrap_or((
        Position::zero(),
        VelocityVector::zero(),
    ));

    let total_loss = Length::new((elements.semi_major_axis.value() - a).max(0.0));
    let remaining_life = if is_deorbited {
        deorbit_offset
    } else {
        estimate_orbit_lifetime(
            &final_elements,
            atmospheric_surface_density,
            atmospheric_scale_height,
            atmospheric_boundary_altitude,
            ballistic_coefficient_inverse,
            planet_radius,
            mu,
            harmonics
        )
    };

    SecularOrbitDecayPrediction::new(
        *elements,
        final_elements,
        pos,
        vel,
        Duration::new(elapsed),
        remaining_life,
        is_deorbited,
        deorbit_offset,
        total_loss
    )
}

pub fn estimate_orbit_lifetime(
    elements: &OsculatingElements,
    atmospheric_surface_density: Density,
    atmospheric_scale_height: Length,
    atmospheric_boundary_altitude: Length,
    ballistic_coefficient_inverse: f64,
    planet_radius: Length,
    mu: GravitationalParameter,
    _harmonics: &ZonalHarmonics
) -> Option<Duration> {
    let a = elements.semi_major_axis.value();
    let e = elements.eccentricity.clamp(0.0, 0.999);
    let r_planet = planet_radius.value();
    let h_scale = atmospheric_scale_height.value();
    let beta = ballistic_coefficient_inverse;
    let mu_val = mu.value();

    if a <= r_planet || beta <= 0.0 || h_scale <= 0.0 || mu_val <= 0.0 {
        return Some(Duration::new(0.0));
    }

    let rho_p = density_at_periapsis(
        Length::new(a),
        e,
        planet_radius,
        atmospheric_surface_density,
        atmospheric_scale_height,
        atmospheric_boundary_altitude
    ).value();

    if rho_p <= 1e-18 {
        return None;
    }

    let c = ((a * e) / h_scale).max(0.0);
    let sqrt_mu_a = (mu_val * a).sqrt();

    let da_dt = if c < 1e-4 {
        beta * sqrt_mu_a * rho_p
    } else {
        let i0 = bessel_i0_scaled(c);
        let i1 = bessel_i1_scaled(c);
        beta * sqrt_mu_a * rho_p * (i0 + 2.0 * e * i1)
    };

    if da_dt <= SECULAR_DECAY_EPSILON {
        return None;
    }

    let lifetime = h_scale / da_dt;
    if lifetime.is_finite() && lifetime > 0.0 {
        Some(Duration::new(lifetime))
    } else {
        None
    }
}
