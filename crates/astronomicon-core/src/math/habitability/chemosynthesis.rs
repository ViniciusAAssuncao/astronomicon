use crate::chemistry::thermochemistry::{
    hydrogen_sulfide_oxidation, hydrogen_sulfide_partial_oxidation, iron_oxidation, methanogenesis,
    methanotrophy,
};
use crate::math::hydrosphere::hydrostatic_pressure_at_depth;
use crate::math::volcanism::outgassing::{volcanic_outgassing_fluxes, VolcanicGasOutgassingRates};
use crate::units::{Acceleration, Density, Length, Luminosity, MassRate, Temperature};
use serde::{Deserialize, Serialize};

pub const DEFAULT_CHEMOLITHOTROPHIC_EFFICIENCY: f64 = 0.05;
pub const H2S_MOLAR_MASS_KG_PER_MOL: f64 = 0.03408;
pub const CH4_MOLAR_MASS_KG_PER_MOL: f64 = 0.01604;
pub const H2_MOLAR_MASS_KG_PER_MOL: f64 = 0.002016;
pub const FE2_MOLAR_MASS_KG_PER_MOL: f64 = 0.055845;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ChemosyntheticPathways {
    pub h2s_oxidation_power: Luminosity,
    pub methanogenesis_power: Luminosity,
    pub methanotrophy_power: Luminosity,
    pub iron_oxidation_power: Luminosity,
    pub total_chemical_power: Luminosity,
    pub net_biomass_production_power: Luminosity,
    pub conversion_efficiency: f64,
}

impl ChemosyntheticPathways {
    pub fn new(
        h2s_oxidation_power: Luminosity,
        methanogenesis_power: Luminosity,
        methanotrophy_power: Luminosity,
        iron_oxidation_power: Luminosity,
        total_chemical_power: Luminosity,
        net_biomass_production_power: Luminosity,
        conversion_efficiency: f64,
    ) -> Self {
        Self {
            h2s_oxidation_power,
            methanogenesis_power,
            methanotrophy_power,
            iron_oxidation_power,
            total_chemical_power,
            net_biomass_production_power,
            conversion_efficiency,
        }
    }

    pub fn h2s_oxidation_power(&self) -> Luminosity {
        self.h2s_oxidation_power
    }

    pub fn methanogenesis_power(&self) -> Luminosity {
        self.methanogenesis_power
    }

    pub fn methanotrophy_power(&self) -> Luminosity {
        self.methanotrophy_power
    }

    pub fn iron_oxidation_power(&self) -> Luminosity {
        self.iron_oxidation_power
    }

    pub fn total_chemical_power(&self) -> Luminosity {
        self.total_chemical_power
    }

    pub fn net_biomass_production_power(&self) -> Luminosity {
        self.net_biomass_production_power
    }

    pub fn conversion_efficiency(&self) -> f64 {
        self.conversion_efficiency
    }
}

pub fn molar_flow_rate(mass_rate: MassRate, molar_mass_kg_per_mol: f64) -> f64 {
    let m_dot = mass_rate.value();
    let mu = molar_mass_kg_per_mol;
    if !m_dot.is_finite() || m_dot <= 0.0 || !mu.is_finite() || mu <= 0.0 {
        0.0
    } else {
        m_dot / mu
    }
}

pub fn reaction_power(molar_rate_mol_per_s: f64, delta_g_j_per_mol: f64) -> Luminosity {
    if !molar_rate_mol_per_s.is_finite()
        || molar_rate_mol_per_s <= 0.0
        || !delta_g_j_per_mol.is_finite()
    {
        return Luminosity::new(0.0);
    }
    if delta_g_j_per_mol >= 0.0 {
        return Luminosity::new(0.0);
    }
    let energy_per_mol = -delta_g_j_per_mol;
    let power_watts = molar_rate_mol_per_s * energy_per_mol;
    Luminosity::new(power_watts.max(0.0))
}

pub fn h2s_oxidation_power(h2s_mass_rate: MassRate, temperature: Temperature) -> Luminosity {
    let n_dot = molar_flow_rate(h2s_mass_rate, H2S_MOLAR_MASS_KG_PER_MOL);
    let thermo = hydrogen_sulfide_oxidation(temperature);
    reaction_power(n_dot, thermo.delta_g_at_temperature)
}

pub fn h2s_partial_oxidation_power(h2s_mass_rate: MassRate, temperature: Temperature) -> Luminosity {
    let n_dot = molar_flow_rate(h2s_mass_rate, H2S_MOLAR_MASS_KG_PER_MOL);
    let thermo = hydrogen_sulfide_partial_oxidation(temperature);
    reaction_power(n_dot, thermo.delta_g_at_temperature)
}

pub fn methanotrophy_power(ch4_mass_rate: MassRate, temperature: Temperature) -> Luminosity {
    let n_dot = molar_flow_rate(ch4_mass_rate, CH4_MOLAR_MASS_KG_PER_MOL);
    let thermo = methanotrophy(temperature);
    reaction_power(n_dot, thermo.delta_g_at_temperature)
}

pub fn methanogenesis_power(h2_mass_rate: MassRate, temperature: Temperature) -> Luminosity {
    let n_dot_h2 = molar_flow_rate(h2_mass_rate, H2_MOLAR_MASS_KG_PER_MOL);
    let n_dot_reaction = n_dot_h2 / 4.0;
    let thermo = methanogenesis(temperature);
    reaction_power(n_dot_reaction, thermo.delta_g_at_temperature)
}

pub fn iron_oxidation_power(fe2_mass_rate: MassRate, temperature: Temperature) -> Luminosity {
    let n_dot_fe = molar_flow_rate(fe2_mass_rate, FE2_MOLAR_MASS_KG_PER_MOL);
    let n_dot_reaction = n_dot_fe / 4.0;
    let thermo = iron_oxidation(temperature);
    reaction_power(n_dot_reaction, thermo.delta_g_at_temperature)
}

pub fn evaluate_surface_chemosynthesis(
    outgassing_rates: &VolcanicGasOutgassingRates,
    atmospheric_ch4_mass_rate: Option<MassRate>,
    temperature: Temperature,
    conversion_efficiency: Option<f64>,
) -> ChemosyntheticPathways {
    let eff = conversion_efficiency
        .unwrap_or(DEFAULT_CHEMOLITHOTROPHIC_EFFICIENCY)
        .clamp(0.0, 1.0);
    let p_h2s = h2s_oxidation_power(outgassing_rates.h2s, temperature);
    let p_ch4 = atmospheric_ch4_mass_rate
        .map(|r| methanotrophy_power(r, temperature))
        .unwrap_or_else(|| Luminosity::new(0.0));
    let p_methanogenesis = Luminosity::new(0.0);
    let p_fe = Luminosity::new(0.0);

    let total = p_h2s.value() + p_ch4.value() + p_methanogenesis.value() + p_fe.value();
    let biomass = total * eff;

    ChemosyntheticPathways::new(
        p_h2s,
        p_methanogenesis,
        p_ch4,
        p_fe,
        Luminosity::new(total),
        Luminosity::new(biomass),
        eff,
    )
}

pub fn evaluate_subsurface_ocean_chemosynthesis(
    magma_extrusion_rate: MassRate,
    mantle_hydration: f64,
    c_o_ratio: f64,
    sulfur_mass_fraction: f64,
    liquid_density: Density,
    gravity: Acceleration,
    total_depth: Length,
    temperature: Temperature,
    conversion_efficiency: Option<f64>,
) -> ChemosyntheticPathways {
    let eff = conversion_efficiency
        .unwrap_or(DEFAULT_CHEMOLITHOTROPHIC_EFFICIENCY)
        .clamp(0.0, 1.0);
    let p_hydrostatic = hydrostatic_pressure_at_depth(liquid_density, gravity, total_depth);
    let seafloor_outgassing = volcanic_outgassing_fluxes(
        magma_extrusion_rate,
        mantle_hydration,
        c_o_ratio,
        sulfur_mass_fraction,
        p_hydrostatic,
    );

    let p_h2s = h2s_oxidation_power(seafloor_outgassing.h2s, temperature);
    let p_methanogenesis = methanogenesis_power(
        MassRate::new(seafloor_outgassing.h2s.value() * 0.1),
        temperature,
    );
    let p_ch4 = Luminosity::new(0.0);
    let p_fe = Luminosity::new(0.0);

    let total = p_h2s.value() + p_methanogenesis.value() + p_ch4.value() + p_fe.value();
    let biomass = total * eff;

    ChemosyntheticPathways::new(
        p_h2s,
        p_methanogenesis,
        p_ch4,
        p_fe,
        Luminosity::new(total),
        Luminosity::new(biomass),
        eff,
    )
}
