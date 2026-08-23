use crate::units::constants::{
    BOLTZMANN_CONSTANT, GRAVITATIONAL_CONSTANT, PLANCK_CONSTANT, PROTON_MASS, SPEED_OF_LIGHT,
    STEFAN_BOLTZMANN_CONSTANT, THOMSON_CROSS_SECTION, THORNE_SPIN_LIMIT,
};
use crate::units::{
    Angle, AngularVelocity, Density, Duration, Length, Luminosity, Mass, MassRate, Speed,
    Temperature,
};
use std::f64::consts::{PI, TAU};

pub fn gravitational_radius(mass: Mass) -> Length {
    let m = mass.value();
    if m <= 0.0 || !m.is_finite() {
        return Length::new(0.0);
    }
    let rg = (GRAVITATIONAL_CONSTANT * m) / (SPEED_OF_LIGHT * SPEED_OF_LIGHT);
    Length::new(rg)
}

pub fn schwarzschild_radius(mass: Mass) -> Length {
    Length::new(2.0 * gravitational_radius(mass).value())
}

pub fn event_horizon_radius(mass: Mass, dimensionless_spin: f64) -> Length {
    let rg = gravitational_radius(mass).value();
    if rg <= 0.0 {
        return Length::new(0.0);
    }
    let a_star = dimensionless_spin.clamp(0.0, THORNE_SPIN_LIMIT);
    let term = (1.0 - a_star * a_star).max(0.0).sqrt();
    Length::new(rg * (1.0 + term))
}

pub fn ergosphere_radius(mass: Mass, dimensionless_spin: f64, latitude: Angle) -> Length {
    let rg = gravitational_radius(mass).value();
    if rg <= 0.0 {
        return Length::new(0.0);
    }
    let a_star = dimensionless_spin.clamp(0.0, THORNE_SPIN_LIMIT);
    let sin_lat = latitude.value().sin();
    let term = (1.0 - a_star * a_star * sin_lat * sin_lat).max(0.0).sqrt();
    Length::new(rg * (1.0 + term))
}

pub fn photon_sphere_radius_prograde(mass: Mass, dimensionless_spin: f64) -> Length {
    let rg = gravitational_radius(mass).value();
    if rg <= 0.0 {
        return Length::new(0.0);
    }
    let a_star = dimensionless_spin.clamp(0.0, THORNE_SPIN_LIMIT);
    let term = (2.0 / 3.0) * (-a_star).acos();
    Length::new(2.0 * rg * (1.0 + term.cos()))
}

pub fn photon_sphere_radius_retrograde(mass: Mass, dimensionless_spin: f64) -> Length {
    let rg = gravitational_radius(mass).value();
    if rg <= 0.0 {
        return Length::new(0.0);
    }
    let a_star = dimensionless_spin.clamp(0.0, THORNE_SPIN_LIMIT);
    let term = (2.0 / 3.0) * a_star.acos();
    Length::new(2.0 * rg * (1.0 + term.cos()))
}

pub fn photon_sphere_radii(mass: Mass, dimensionless_spin: f64) -> (Length, Length) {
    (
        photon_sphere_radius_prograde(mass, dimensionless_spin),
        photon_sphere_radius_retrograde(mass, dimensionless_spin),
    )
}

pub fn isco_radius_prograde(mass: Mass, dimensionless_spin: f64) -> Length {
    let rg = gravitational_radius(mass).value();
    if rg <= 0.0 {
        return Length::new(0.0);
    }
    let a = dimensionless_spin.clamp(0.0, THORNE_SPIN_LIMIT);
    let z1 = 1.0 + (1.0 - a * a).cbrt() * ((1.0 + a).cbrt() + (1.0 - a).cbrt());
    let z2 = (3.0 * a * a + z1 * z1).sqrt();
    let term = ((3.0 - z1) * (3.0 + z1 + 2.0 * z2)).max(0.0).sqrt();
    let r_isco = rg * (3.0 + z2 - term);
    Length::new(r_isco)
}

pub fn isco_radius_retrograde(mass: Mass, dimensionless_spin: f64) -> Length {
    let rg = gravitational_radius(mass).value();
    if rg <= 0.0 {
        return Length::new(0.0);
    }
    let a = dimensionless_spin.clamp(0.0, THORNE_SPIN_LIMIT);
    let z1 = 1.0 + (1.0 - a * a).cbrt() * ((1.0 + a).cbrt() + (1.0 - a).cbrt());
    let z2 = (3.0 * a * a + z1 * z1).sqrt();
    let term = ((3.0 - z1) * (3.0 + z1 + 2.0 * z2)).max(0.0).sqrt();
    let r_isco = rg * (3.0 + z2 + term);
    Length::new(r_isco)
}

pub fn isco_radii(mass: Mass, dimensionless_spin: f64) -> (Length, Length) {
    (
        isco_radius_prograde(mass, dimensionless_spin),
        isco_radius_retrograde(mass, dimensionless_spin),
    )
}

pub fn eddington_luminosity(mass: Mass) -> Luminosity {
    let m = mass.value();
    if m <= 0.0 || !m.is_finite() {
        return Luminosity::new(0.0);
    }
    let l_edd = (4.0 * PI * GRAVITATIONAL_CONSTANT * SPEED_OF_LIGHT * PROTON_MASS * m)
        / THOMSON_CROSS_SECTION;
    if !l_edd.is_finite() || l_edd < 0.0 {
        Luminosity::new(0.0)
    } else {
        Luminosity::new(l_edd)
    }
}

pub fn hawking_temperature(mass: Mass, dimensionless_spin: f64) -> Temperature {
    let m = mass.value();
    if m <= 0.0 || !m.is_finite() {
        return Temperature::new(0.0);
    }
    let a_star = dimensionless_spin.clamp(0.0, THORNE_SPIN_LIMIT);
    let sqrt_term = (1.0 - a_star * a_star).max(0.0).sqrt();
    let num = PLANCK_CONSTANT * SPEED_OF_LIGHT.powi(3) * sqrt_term;
    let den = 8.0 * PI * PI * BOLTZMANN_CONSTANT * GRAVITATIONAL_CONSTANT * m * (1.0 + sqrt_term);
    if den <= 0.0 || !den.is_finite() {
        return Temperature::new(0.0);
    }
    let t_h = num / den;
    if !t_h.is_finite() || t_h < 0.0 {
        Temperature::new(0.0)
    } else {
        Temperature::new(t_h)
    }
}

pub fn hawking_luminosity(mass: Mass, dimensionless_spin: f64) -> Luminosity {
    let m = mass.value();
    if m <= 0.0 || !m.is_finite() {
        return Luminosity::new(0.0);
    }
    let a_star = dimensionless_spin.clamp(0.0, THORNE_SPIN_LIMIT);
    let rg = gravitational_radius(mass).value();
    let sqrt_term = (1.0 - a_star * a_star).max(0.0).sqrt();
    let area = 8.0 * PI * rg * rg * (1.0 + sqrt_term);
    let t_h = hawking_temperature(mass, dimensionless_spin).value();
    let l_h = STEFAN_BOLTZMANN_CONSTANT * area * t_h.powi(4);
    if !l_h.is_finite() || l_h < 0.0 {
        Luminosity::new(0.0)
    } else {
        Luminosity::new(l_h)
    }
}

pub fn radiative_efficiency(dimensionless_spin: f64) -> f64 {
    let a = dimensionless_spin.clamp(0.0, THORNE_SPIN_LIMIT);
    let z1 = 1.0 + (1.0 - a * a).cbrt() * ((1.0 + a).cbrt() + (1.0 - a).cbrt());
    let z2 = (3.0 * a * a + z1 * z1).sqrt();
    let term = ((3.0 - z1) * (3.0 + z1 + 2.0 * z2)).max(0.0).sqrt();
    let r = 3.0 + z2 - term;
    if r <= 0.0 || !r.is_finite() {
        return 0.0572;
    }
    let r_sqrt = r.sqrt();
    let num = r * r_sqrt - 2.0 * r_sqrt + a;
    let den_inside = r * r_sqrt - 3.0 * r_sqrt + 2.0 * a;
    if den_inside <= 0.0 {
        return 1.0 - 1.0 / 3.0_f64.sqrt();
    }
    let den = r.powf(0.75) * den_inside.sqrt();
    if den <= 0.0 || !den.is_finite() {
        return 0.0572;
    }
    let energy = num / den;
    (1.0 - energy).clamp(0.0, 1.0)
}

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

pub fn horizon_angular_velocity(mass: Mass, dimensionless_spin: f64) -> AngularVelocity {
    let rg = gravitational_radius(mass).value();
    if rg <= 0.0 {
        return AngularVelocity::new(0.0);
    }
    let a_star = dimensionless_spin.clamp(0.0, THORNE_SPIN_LIMIT);
    if a_star <= 0.0 {
        return AngularVelocity::new(0.0);
    }
    let term = (1.0 - a_star * a_star).max(0.0).sqrt();
    let r_plus = rg * (1.0 + term);
    let omega_h = (a_star * SPEED_OF_LIGHT) / (2.0 * r_plus);
    AngularVelocity::new(omega_h)
}

pub fn horizon_rotation_period(mass: Mass, dimensionless_spin: f64) -> Option<Duration> {
    let omega = horizon_angular_velocity(mass, dimensionless_spin).value();
    if omega <= 0.0 || !omega.is_finite() {
        None
    } else {
        Some(Duration::new(TAU / omega))
    }
}

pub fn dimensionless_spin_from_angular_velocity(
    mass: Mass,
    angular_velocity: AngularVelocity,
) -> f64 {
    let rg = gravitational_radius(mass).value();
    let omega = angular_velocity.value().abs();
    if rg <= 0.0 || omega <= 0.0 || !rg.is_finite() || !omega.is_finite() {
        return 0.0;
    }
    let w = (omega * rg) / SPEED_OF_LIGHT;
    if w >= 0.5 {
        return THORNE_SPIN_LIMIT;
    }
    let a_star = (4.0 * w) / (1.0 + 4.0 * w * w);
    a_star.clamp(0.0, THORNE_SPIN_LIMIT)
}

pub fn dimensionless_spin_from_rotation_period(
    mass: Mass,
    rotation_period: Duration,
) -> f64 {
    let period = rotation_period.value();
    if period <= 0.0 || !period.is_finite() {
        return 0.0;
    }
    let omega_h = AngularVelocity::new(TAU / period);
    dimensionless_spin_from_angular_velocity(mass, omega_h)
}

pub fn dimensionless_spin(mass: Mass, rotation_period: Duration) -> f64 {
    dimensionless_spin_from_rotation_period(mass, rotation_period)
}