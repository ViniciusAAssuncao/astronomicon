use crate::units::constants::{STEFAN_BOLTZMANN_CONSTANT, UNIVERSAL_GAS_CONSTANT};
use crate::units::{Irradiance, MassRate, MolarMass, Pressure, Speed, Temperature};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CometaryVolatile {
    Water,
    CarbonDioxide,
    CarbonMonoxide,
}

pub fn latent_heat_of_sublimation(volatile: CometaryVolatile) -> f64 {
    match volatile {
        CometaryVolatile::Water => 2.834e6,
        CometaryVolatile::CarbonDioxide => 5.73e5,
        CometaryVolatile::CarbonMonoxide => 2.90e5,
    }
}

pub fn volatile_molar_mass(volatile: CometaryVolatile) -> MolarMass {
    match volatile {
        CometaryVolatile::Water => MolarMass::new(0.01801528),
        CometaryVolatile::CarbonDioxide => MolarMass::new(0.04401),
        CometaryVolatile::CarbonMonoxide => MolarMass::new(0.02801),
    }
}

pub fn volatile_reference_parameters(volatile: CometaryVolatile) -> (f64, f64) {
    match volatile {
        CometaryVolatile::Water => (373.15, 101325.0),
        CometaryVolatile::CarbonDioxide => (194.65, 101325.0),
        CometaryVolatile::CarbonMonoxide => (81.65, 101325.0),
    }
}

pub fn volatile_vapor_pressure(volatile: CometaryVolatile, temperature: Temperature) -> Pressure {
    let t = temperature.value();
    if t <= 0.0 || !t.is_finite() {
        return Pressure::new(0.0);
    }

    let l_s = latent_heat_of_sublimation(volatile);
    let mu = volatile_molar_mass(volatile).value();
    let (t0, p0) = volatile_reference_parameters(volatile);

    let exponent = -(l_s * mu / UNIVERSAL_GAS_CONSTANT) * (1.0 / t - 1.0 / t0);
    if exponent < -100.0 {
        Pressure::new(0.0)
    } else if exponent > 100.0 {
        Pressure::new(p0 * 100.0_f64.exp())
    } else {
        Pressure::new(p0 * exponent.exp())
    }
}

pub fn sublimation_mass_flux(volatile: CometaryVolatile, temperature: Temperature) -> f64 {
    let t = temperature.value();
    if t <= 0.0 || !t.is_finite() {
        return 0.0;
    }

    let p_vap = volatile_vapor_pressure(volatile, temperature).value();
    if p_vap <= 0.0 {
        return 0.0;
    }

    let mu = volatile_molar_mass(volatile).value();
    let factor = (mu / (2.0 * PI * UNIVERSAL_GAS_CONSTANT * t)).sqrt();
    let z = p_vap * factor;

    if !z.is_finite() || z <= 0.0 {
        0.0
    } else {
        z
    }
}

pub fn sublimation_equilibrium(
    insolation: Irradiance,
    bond_albedo: f64,
    emissivity: f64,
    volatile: CometaryVolatile,
) -> (Temperature, f64) {
    let f_in = insolation.value();
    if f_in <= 0.0 || !f_in.is_finite() {
        return (Temperature::new(0.0), 0.0);
    }

    let a = bond_albedo.clamp(0.0, 1.0);
    let eps = emissivity.clamp(0.01, 1.0);
    let f_abs = (1.0 - a) * f_in;
    let l_s = latent_heat_of_sublimation(volatile);

    let mut t_low = 1.0;
    let mut t_high = 1000.0;

    for _ in 0..60 {
        let t_mid = 0.5 * (t_low + t_high);
        let z = sublimation_mass_flux(volatile, Temperature::new(t_mid));
        let f_rad = eps * STEFAN_BOLTZMANN_CONSTANT * t_mid.powi(4);
        let f_sub = z * l_s;
        let f_tot = f_rad + f_sub;

        if f_tot < f_abs {
            t_low = t_mid;
        } else {
            t_high = t_mid;
        }
    }

    let t_eq = 0.5 * (t_low + t_high);
    let z_eq = sublimation_mass_flux(volatile, Temperature::new(t_eq));
    (Temperature::new(t_eq), z_eq)
}

pub fn cometary_gas_production_rate(
    surface_area_m2: f64,
    active_fraction: f64,
    insolation: Irradiance,
    bond_albedo: f64,
    volatile: CometaryVolatile,
) -> (MassRate, f64) {
    if surface_area_m2 <= 0.0 || !surface_area_m2.is_finite() {
        return (MassRate::new(0.0), 0.0);
    }

    let act = active_fraction.clamp(0.0, 1.0);
    let effective_area = surface_area_m2 * act;

    let avg_insolation = Irradiance::new(insolation.value() * 0.25);
    let (_, z) = sublimation_equilibrium(avg_insolation, bond_albedo, 0.9, volatile);

    let q_mass = z * effective_area;
    let mu = volatile_molar_mass(volatile).value();
    let avogadro = 6.02214076e23;
    let q_molecules = if mu > 0.0 {
        (q_mass / mu) * avogadro
    } else {
        0.0
    };

    (MassRate::new(q_mass), q_molecules)
}

pub fn thermal_gas_expansion_speed(temperature: Temperature, volatile: CometaryVolatile) -> Speed {
    let t = temperature.value();
    let mu = volatile_molar_mass(volatile).value();
    if t <= 0.0 || mu <= 0.0 || !t.is_finite() {
        return Speed::new(0.0);
    }

    let v = (8.0 * UNIVERSAL_GAS_CONSTANT * t / (PI * mu)).sqrt();
    if !v.is_finite() || v <= 0.0 {
        Speed::new(0.0)
    } else {
        Speed::new(v)
    }
}
