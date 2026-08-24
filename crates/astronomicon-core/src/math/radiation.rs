use crate::units::constants::{
    BOLTZMANN_CONSTANT, GALACTIC_COSMIC_RAY_BACKGROUND_DOSE, PLANCK_CONSTANT, SPEED_OF_LIGHT,
    STEFAN_BOLTZMANN_CONSTANT, WIEN_DISPLACEMENT_CONSTANT,
};
use crate::units::{
    Acceleration, Angle, Density, Energy, Irradiance, Length, MagneticDipoleMoment,
    MagneticRigidity, MassAttenuationCoefficient, Pressure, RadiationDose, Speed, Temperature,
    Wavelength,
};

pub fn peak_wavelength(temperature: Temperature) -> Wavelength {
    let t = temperature.value();
    if t <= 0.0 || !t.is_finite() {
        return Wavelength::new(0.0);
    }
    Wavelength::new(WIEN_DISPLACEMENT_CONSTANT / t)
}

pub fn planck_spectral_radiance(wavelength: Wavelength, temperature: Temperature) -> f64 {
    let lambda = wavelength.value();
    let t = temperature.value();

    if lambda <= 0.0 || t <= 0.0 || !lambda.is_finite() || !t.is_finite() {
        return 0.0;
    }

    let h = PLANCK_CONSTANT;
    let c = SPEED_OF_LIGHT;
    let k_b = BOLTZMANN_CONSTANT;

    let exponent = (h * c) / (lambda * k_b * t);
    if !exponent.is_finite() || exponent > 700.0 {
        return 0.0;
    }

    let exp_term = exponent.exp() - 1.0;
    if exp_term <= 0.0 || !exp_term.is_finite() {
        return 0.0;
    }

    let numerator = 2.0 * h * c * c;
    let denominator = lambda.powi(5) * exp_term;

    if denominator <= 0.0 || !denominator.is_finite() {
        return 0.0;
    }

    let radiance = numerator / denominator;
    if !radiance.is_finite() || radiance < 0.0 {
        0.0
    } else {
        radiance
    }
}

pub fn cmb_energy_density(temperature: Temperature) -> f64 {
    let t = temperature.value();
    if t <= 0.0 || !t.is_finite() {
        return 0.0;
    }

    let u = (4.0 * STEFAN_BOLTZMANN_CONSTANT / SPEED_OF_LIGHT) * t.powi(4);
    if !u.is_finite() || u < 0.0 { 0.0 } else { u }
}

pub fn stellar_particle_flux(wind_density: Density, terminal_speed: Speed) -> Irradiance {
    let rho = wind_density.value();
    let v = terminal_speed.value();

    if rho <= 0.0 || v <= 0.0 || !rho.is_finite() || !v.is_finite() {
        return Irradiance::new(0.0);
    }

    let flux = 0.5 * rho * v * v * v;
    if !flux.is_finite() || flux < 0.0 {
        Irradiance::new(0.0)
    } else {
        Irradiance::new(flux)
    }
}

pub fn galactic_cosmic_ray_background() -> RadiationDose {
    RadiationDose::new(GALACTIC_COSMIC_RAY_BACKGROUND_DOSE)
}

pub fn cutoff_rigidity(
    dipole_moment: MagneticDipoleMoment,
    radius: Length,
    latitude: Angle,
) -> MagneticRigidity {
    let m = dipole_moment.value();
    let r = radius.value();
    let lat = latitude.value();

    if m <= 0.0 || r <= 0.0 || !m.is_finite() || !r.is_finite() || !lat.is_finite() {
        return MagneticRigidity::new(0.0);
    }

    let cos_lat = lat.cos();
    let cos4 = cos_lat * cos_lat * cos_lat * cos_lat;
    let rigidity = (m / (4.0 * r * r)) * cos4;

    if !rigidity.is_finite() || rigidity < 0.0 {
        MagneticRigidity::new(0.0)
    } else {
        MagneticRigidity::new(rigidity)
    }
}

pub fn magnetosphere_shielding_factor(
    cutoff_rigidity: MagneticRigidity,
    particle_kinetic_energy: Energy,
) -> f64 {
    let rc = cutoff_rigidity.value();
    let e = particle_kinetic_energy.value();

    if e <= 0.0 || !e.is_finite() {
        return 0.0;
    }

    if rc <= 0.0 || !rc.is_finite() {
        return 1.0;
    }

    let ratio = rc / e;
    let transmissivity = 1.0 / (1.0 + ratio.powi(4));

    transmissivity.clamp(0.0, 1.0)
}

pub fn atmospheric_mass_column(surface_pressure: Pressure, gravity: Acceleration) -> f64 {
    let p = surface_pressure.value();
    let g = gravity.value();

    if p <= 0.0 || g <= 0.0 || !p.is_finite() || !g.is_finite() {
        return 0.0;
    }

    p / g
}

pub fn atmospheric_transmission(
    mass_column: f64,
    mean_attenuation_coeff: MassAttenuationCoefficient,
) -> f64 {
    let x = mass_column;
    let mu = mean_attenuation_coeff.value();

    if x <= 0.0 || mu <= 0.0 || !x.is_finite() || !mu.is_finite() {
        return 1.0;
    }

    let optical_depth = mu * x;
    if optical_depth < 0.0 {
        return 1.0;
    }

    (-optical_depth).exp().clamp(0.0, 1.0)
}
