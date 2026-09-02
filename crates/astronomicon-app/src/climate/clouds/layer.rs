use astronomicon_core::chemistry::solvent::SolventProperties;
use astronomicon_core::domain::Atmosphere;
use astronomicon_core::math::atmosphere::ideal_gas_density;
use astronomicon_core::math::climate::temperature_at_altitude;
use astronomicon_core::math::clouds::{
    GlaciationState, adiabatic_condensate_density, glaciation_state_from_ice_fraction,
    ice_condensate_density, ice_fraction_at_altitude, layer_critical_relative_humidity,
    liquid_condensate_density, mixing_ratio_at_altitude, relative_humidity_at_altitude,
    sundqvist_cloud_fraction_with_ccn,
};
use astronomicon_core::math::thermodynamics::{
    saturation_mixing_ratio, saturation_vapor_pressure,
};
use astronomicon_core::units::{Density, Length, MolarMass, Temperature};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CloudLayerDiagnostic {
    pub base_altitude: Length,
    pub top_altitude: Length,
    pub representative_altitude: Length,
    pub cloud_fraction: f64,
    pub relative_humidity: f64,
    pub critical_relative_humidity: f64,
    pub liquid_water_content: Density,
    pub ice_water_content: Density,
    pub ice_fraction: f64,
    pub glaciation_state: GlaciationState,
}

pub fn evaluate_cloud_layer(
    atmosphere: &Atmosphere,
    z_base: Length,
    z_top: Length,
    z_mid: Length,
    surface_humidity: f64,
    surf_temp: Temperature,
    tropopause_altitude: Length,
    skin_temperature: Temperature,
    scale_h: Length,
    atm_molar_mass: MolarMass,
    solvent_props: &SolventProperties,
    solvent_molar_mass: MolarMass,
    freezing_level: Length,
) -> CloudLayerDiagnostic {
    let rh = relative_humidity_at_altitude(surface_humidity, z_mid, tropopause_altitude);
    let press_z = atmosphere.pressure_at_altitude(z_mid, scale_h);
    let temp_z = temperature_at_altitude(surf_temp, z_mid, atmosphere.lapse_rate());
    let temp_z_clamped = if temp_z.value() < skin_temperature.value() {
        skin_temperature
    } else {
        temp_z
    };

    let surf_press = atmosphere.surface_pressure();
    let sigma = if surf_press.value() > 0.0 {
        (press_z.value() / surf_press.value()).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let rh_crit = layer_critical_relative_humidity(sigma);
    let c_layer = sundqvist_cloud_fraction_with_ccn(
        rh,
        rh_crit,
        atmosphere.cloud_condensation_nuclei_factor(),
    );

    let rho_air = ideal_gas_density(press_z, atm_molar_mass, temp_z_clamped);
    let r_surf = mixing_ratio_at_altitude(
        surface_humidity,
        surf_temp,
        surf_press,
        solvent_props,
        solvent_molar_mass,
        atm_molar_mass,
    );
    let p_sat_z = saturation_vapor_pressure(temp_z_clamped, solvent_props);
    let r_sat_z = saturation_mixing_ratio(p_sat_z, press_z, solvent_molar_mass, atm_molar_mass);
    let rho_condensate = adiabatic_condensate_density(rho_air, r_surf, r_sat_z);

    let ice_frac = ice_fraction_at_altitude(z_mid, freezing_level);
    let rho_liquid = liquid_condensate_density(rho_condensate, ice_frac);
    let rho_ice = ice_condensate_density(rho_condensate, ice_frac);
    let glaciation = glaciation_state_from_ice_fraction(ice_frac);

    CloudLayerDiagnostic {
        base_altitude: z_base,
        top_altitude: z_top,
        representative_altitude: z_mid,
        cloud_fraction: c_layer,
        relative_humidity: rh,
        critical_relative_humidity: rh_crit,
        liquid_water_content: rho_liquid,
        ice_water_content: rho_ice,
        ice_fraction: ice_frac,
        glaciation_state: glaciation,
    }
}