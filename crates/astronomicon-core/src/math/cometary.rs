use crate::units::constants::{
    ASTRONOMICAL_UNIT, SPEED_OF_LIGHT, STEFAN_BOLTZMANN_CONSTANT, UNIVERSAL_GAS_CONSTANT,
};
use crate::units::{
    Irradiance, Length, MassRate, MolarMass, Pressure, Speed, Temperature,
};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CometaryVolatile {
    Water,
    CarbonDioxide,
    CarbonMonoxide,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CometaryTailStructure {
    pub ion_tail_length: Length,
    pub dust_tail_length: Length,
    pub ion_tail_drag_force_n: f64,
    pub dust_radiation_force_n: f64,
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

pub fn volatile_vapor_pressure(
    volatile: CometaryVolatile,
    temperature: Temperature,
) -> Pressure {
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

pub fn sublimation_mass_flux(
    volatile: CometaryVolatile,
    temperature: Temperature,
) -> f64 {
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
    let q_molecules = if mu > 0.0 { (q_mass / mu) * avogadro } else { 0.0 };

    (MassRate::new(q_mass), q_molecules)
}

pub fn thermal_gas_expansion_speed(
    temperature: Temperature,
    volatile: CometaryVolatile,
) -> Speed {
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

pub fn coma_radius(
    gas_production_rate: MassRate,
    expansion_speed: Speed,
    stellar_wind_dynamic_pressure: Pressure,
    irradiance: Irradiance,
) -> Length {
    let q = gas_production_rate.value();
    let v = expansion_speed.value();
    let p_sw = stellar_wind_dynamic_pressure.value();
    let f_irr = irradiance.value();

    if q <= 0.0 || v <= 0.0 || !q.is_finite() || !v.is_finite() {
        return Length::new(0.0);
    }

    let p_rad = if f_irr > 0.0 && f_irr.is_finite() {
        f_irr / SPEED_OF_LIGHT
    } else {
        0.0
    };

    let total_external_pressure = (p_sw + p_rad).max(1e-18);
    let r_coma = (q * v / (4.0 * PI * total_external_pressure)).sqrt();

    if !r_coma.is_finite() || r_coma <= 0.0 {
        Length::new(0.0)
    } else {
        Length::new(r_coma)
    }
}

pub fn cometary_tail_structure(
    gas_production_rate: MassRate,
    dust_to_gas_mass_ratio: f64,
    _expansion_speed: Speed,
    coma_radius: Length,
    stellar_wind_dynamic_pressure: Pressure,
    stellar_wind_speed: Speed,
    irradiance: Irradiance,
    heliocentric_distance: Length,
) -> CometaryTailStructure {
    let q_gas = gas_production_rate.value();
    let d_to_g = if dust_to_gas_mass_ratio.is_finite() && dust_to_gas_mass_ratio >= 0.0 {
        dust_to_gas_mass_ratio
    } else {
        1.0
    };
    let q_dust = q_gas * d_to_g;
    let v_wind = stellar_wind_speed.value();
    let p_sw = stellar_wind_dynamic_pressure.value();
    let f_irr = irradiance.value();
    let r_h = heliocentric_distance.value();
    let r_coma = coma_radius.value();

    if q_gas <= 0.0 || r_h <= 0.0 || !q_gas.is_finite() || !r_h.is_finite() {
        return CometaryTailStructure {
            ion_tail_length: Length::new(0.0),
            dust_tail_length: Length::new(0.0),
            ion_tail_drag_force_n: 0.0,
            dust_radiation_force_n: 0.0,
        };
    }

    let r_au = r_h / ASTRONOMICAL_UNIT;
    let tau_ion = 1.0e6 * r_au * r_au;
    let ion_length = (v_wind.max(1.0e3) * tau_ion).max(0.0);

    let area_coma = PI * r_coma * r_coma;
    let ion_drag_force = p_sw.max(0.0) * area_coma;

    let dust_grain_radius = 1.0e-6;
    let dust_grain_density = 1000.0;
    let p_rad = if f_irr > 0.0 && f_irr.is_finite() {
        f_irr / SPEED_OF_LIGHT
    } else {
        0.0
    };

    let q_pr = 1.0;
    let a_rad = (3.0 * p_rad * q_pr) / (4.0 * dust_grain_density * dust_grain_radius);
    let tau_dust = 1.5e6;
    let dust_length = 0.5 * a_rad * tau_dust * tau_dust;

    let total_dust_cross_section = if dust_grain_density > 0.0 && dust_grain_radius > 0.0 {
        (3.0 * q_dust * tau_dust) / (4.0 * dust_grain_density * dust_grain_radius)
    } else {
        0.0
    };
    let dust_radiation_force = p_rad * total_dust_cross_section;

    CometaryTailStructure {
        ion_tail_length: Length::new(ion_length),
        dust_tail_length: Length::new(dust_length),
        ion_tail_drag_force_n: ion_drag_force,
        dust_radiation_force_n: dust_radiation_force,
    }
}