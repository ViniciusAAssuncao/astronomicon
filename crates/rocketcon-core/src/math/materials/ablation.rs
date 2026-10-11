use crate::domain::{HeatShieldState, MaterialRecord};
use crate::error::RocketDomainResult;
use astronomicon_core::units::constants::STEFAN_BOLTZMANN_CONSTANT;
use astronomicon_core::units::{
    Density, Duration, HeatFlux, Length, SpecificEnergy, Speed, Temperature,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AblationStepResult {
    pub updated_state: HeatShieldState,
    pub linear_recession_rate: Speed,
    pub mass_loss_rate_per_unit_area: f64,
    pub blowing_reduction_factor: f64,
    pub transmitted_heat_flux: HeatFlux,
    pub is_burned_through: bool,
}

pub fn reradiation_heat_flux(emissivity: f64, surface_temperature: Temperature) -> HeatFlux {
    let eps = emissivity.clamp(0.0, 1.0);
    let t = surface_temperature.value();
    if eps <= 0.0 || t <= 0.0 || !t.is_finite() {
        return HeatFlux::new(0.0);
    }
    let q = eps * STEFAN_BOLTZMANN_CONSTANT * t.powi(4);
    HeatFlux::new(q.max(0.0))
}

pub fn radiative_equilibrium_surface_temperature(
    incident_heat_flux: HeatFlux,
    emissivity: f64,
) -> Temperature {
    let q = incident_heat_flux.value();
    let eps = emissivity.clamp(0.0, 1.0);
    if q <= 0.0 || eps <= 0.0 || !q.is_finite() {
        return Temperature::new(0.0);
    }
    let denom = eps * STEFAN_BOLTZMANN_CONSTANT;
    if denom <= 0.0 {
        return Temperature::new(0.0);
    }
    let t4 = q / denom;
    Temperature::new(t4.max(0.0).powf(0.25))
}

pub fn recession_rate(
    incident_heat_flux: HeatFlux,
    heat_of_ablation: SpecificEnergy,
    material_density: Density,
    recession_onset_temperature: Temperature,
    emissivity: f64,
) -> (Speed, f64) {
    let q_in = incident_heat_flux.value();
    let h_abl = heat_of_ablation.value();
    let rho = material_density.value();

    if q_in <= 0.0 || h_abl <= 0.0 || rho <= 0.0 || !q_in.is_finite() || !h_abl.is_finite() || !rho.is_finite() {
        return (Speed::new(0.0), 0.0);
    }

    let q_rerad_onset = reradiation_heat_flux(emissivity, recession_onset_temperature).value();
    let q_excess = q_in - q_rerad_onset;

    if q_excess <= 0.0 {
        return (Speed::new(0.0), 0.0);
    }

    let mass_loss_rate = q_excess / h_abl;
    let rec_speed = mass_loss_rate / rho;

    (Speed::new(rec_speed), mass_loss_rate)
}

pub fn pyrolysis_blowing_reduction(
    blowing_coefficient: f64,
    mass_loss_rate_per_unit_area: f64,
) -> f64 {
    if blowing_coefficient <= 0.0 || mass_loss_rate_per_unit_area <= 0.0 || !blowing_coefficient.is_finite() || !mass_loss_rate_per_unit_area.is_finite() {
        return 1.0;
    }
    let denom = 1.0 + blowing_coefficient * mass_loss_rate_per_unit_area;
    if denom <= 0.0 {
        1.0
    } else {
        (1.0 / denom).clamp(0.05, 1.0)
    }
}

pub fn update_heat_shield_state(
    current_state: &HeatShieldState,
    material_record: &MaterialRecord,
    incident_heat_flux: HeatFlux,
    dt: Duration,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketDomainResult<AblationStepResult> {
    let dt_s = dt.value();
    let material = material_record.material();
    let emissivity = material.emissivity();
    let ablative_props_opt = material_record.ablative_properties();

    let (recession_speed, mass_loss_rate, blowing_factor, surf_temp) = match ablative_props_opt {
        Some(props) => {
            let (rec_spd, m_dot) = recession_rate(
                incident_heat_flux,
                props.heat_of_ablation(),
                material.density(),
                props.recession_onset_temperature(),
                emissivity,
            );
            let blow_fac = pyrolysis_blowing_reduction(props.pyrolysis_gas_blowing_coefficient(), m_dot);
            let t_recess = props.recession_onset_temperature();
            let t_rad_eq = radiative_equilibrium_surface_temperature(incident_heat_flux, emissivity);
            let actual_t = if m_dot > 0.0 {
                t_recess
            } else {
                t_rad_eq
            };
            (rec_spd, m_dot, blow_fac, actual_t)
        }
        None => {
            let t_rad_eq = radiative_equilibrium_surface_temperature(incident_heat_flux, emissivity);
            (Speed::new(0.0), 0.0, 1.0, t_rad_eq)
        }
    };

    let delta_thickness = if dt_s > 0.0 && dt_s.is_finite() {
        recession_speed.value() * dt_s
    } else {
        0.0
    };

    let new_thickness_val = (current_state.remaining_thickness_m() - delta_thickness).max(0.0);
    let new_thickness = Length::new(new_thickness_val);
    let is_burned_through = new_thickness_val <= 1e-6;

    let updated_state = HeatShieldState::new(
        current_state.vehicle_component_id(),
        new_thickness,
        surf_temp,
        universe_epoch,
        at_epoch,
    )?;

    let transmitted_flux_val = incident_heat_flux.value() * blowing_factor;
    let transmitted_heat_flux = HeatFlux::new(transmitted_flux_val);

    Ok(AblationStepResult {
        updated_state,
        linear_recession_rate: recession_speed,
        mass_loss_rate_per_unit_area: mass_loss_rate,
        blowing_reduction_factor: blowing_factor,
        transmitted_heat_flux,
        is_burned_through,
    })
}