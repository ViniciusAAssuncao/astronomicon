pub use crate::constants::DEFAULT_STRUCTURAL_HULL_EMISSIVITY;
use crate::domain::{AerospaceMaterial, MaterialRecord};
use crate::error::{RocketDomainError, RocketDomainResult};
use crate::math::materials::{
    default_thermal_softening_exponent, effective_yield_strength, is_thermally_stressed_to_failure,
    thermal_stress_from_gradient,
};
use astronomicon_core::math::graybody_equilibrium_temperature;
use astronomicon_core::units::{Length, Luminosity, Pressure, Temperature};

pub fn effective_ga_product(radiators: &[(f64, f64)]) -> f64 {
    radiators
        .iter()
        .filter_map(|&(area, emissivity)| {
            if area > 0.0 && emissivity > 0.0 && area.is_finite() && emissivity.is_finite() {
                Some(area * emissivity)
            } else {
                None
            }
        })
        .sum()
}

pub fn effective_ga_product_with_hull(
    radiators: &[(f64, f64)],
    hull_area_m2: f64,
    hull_emissivity: f64,
) -> f64 {
    let rad_ga = effective_ga_product(radiators);
    let hull_ga = if hull_area_m2 > 0.0
        && hull_emissivity > 0.0
        && hull_area_m2.is_finite()
        && hull_emissivity.is_finite()
    {
        hull_area_m2 * hull_emissivity
    } else {
        0.0
    };
    rad_ga + hull_ga
}

pub fn vehicle_equilibrium_temperature(
    total_heat_input: Luminosity,
    effective_ga: f64,
) -> Temperature {
    graybody_equilibrium_temperature(total_heat_input, effective_ga)
}

pub fn vehicle_equilibrium_temperature_with_aero(
    internal_waste_heat: Luminosity,
    aerodynamic_heat: Luminosity,
    effective_ga: f64,
) -> Temperature {
    let total_heat = Luminosity::new(internal_waste_heat.value() + aerodynamic_heat.value());
    vehicle_equilibrium_temperature(total_heat, effective_ga)
}

pub fn check_material_thermal_structural_limits_with_stress(
    segment_name: &str,
    material: &AerospaceMaterial,
    thickness: Length,
    segment_temperature: Temperature,
    delta_temperature: Option<Temperature>,
    mechanical_stress: Option<Pressure>,
    custom_softening_exponent: Option<f64>,
) -> RocketDomainResult<()> {
    let t_val = segment_temperature.value();
    if !t_val.is_finite() || t_val <= 0.0 {
        return Ok(());
    }

    let t_melt = material.melting_point();
    let max_service = material.max_service_temperature();

    if let Some(mp) = t_melt {
        if t_val >= mp.value() {
            return Err(RocketDomainError::StructuralFailure {
                reason: format!(
                    "Segment '{}' using material '{}' melted: temperature {:.2} K reached or exceeded melting point {:.2} K",
                    segment_name,
                    material.name(),
                    t_val,
                    mp.value()
                ),
            });
        }
    } else if t_val > max_service.value() {
        return Err(RocketDomainError::StructuralFailure {
            reason: format!(
                "Segment '{}' using material '{}' exceeded maximum service temperature: temperature {:.2} K > max service temperature {:.2} K",
                segment_name,
                material.name(),
                t_val,
                max_service.value()
            ),
        });
    }

    let ref_melting = t_melt.unwrap_or(max_service);
    let exponent = custom_softening_exponent
        .unwrap_or_else(|| default_thermal_softening_exponent(material.material_class()));

    let eff_yield = effective_yield_strength(
        material.base_yield_strength(),
        segment_temperature,
        ref_melting,
        exponent,
    );

    if eff_yield.value() <= 0.0 {
        return Err(RocketDomainError::StructuralFailure {
            reason: format!(
                "Segment '{}' using material '{}' suffered complete loss of structural strength at temperature {:.2} K",
                segment_name,
                material.name(),
                t_val
            ),
        });
    }

    let delta_t = delta_temperature.unwrap_or_else(|| {
        let t_ref = 293.15;
        Temperature::new((t_val - t_ref).max(0.0))
    });

    let thermal_stress = thermal_stress_from_gradient(
        material.thermal_expansion_coefficient_per_k(),
        material.youngs_modulus(),
        delta_t,
    );

    let mech_stress = mechanical_stress.unwrap_or(Pressure::new(0.0));
    let total_stress = thermal_stress + mech_stress;

    if is_thermally_stressed_to_failure(total_stress, eff_yield) {
        let failure_mode = if mech_stress.value() > 0.0 && thermal_stress.value() > 0.0 {
            "combined thermal and mechanical stress"
        } else if thermal_stress.value() > 0.0 {
            "thermal stress"
        } else {
            "loss of strength"
        };

        return Err(RocketDomainError::StructuralFailure {
            reason: format!(
                "Segment '{}' (thickness {:.4} m) using material '{}' failed under {}: total stress {:.2} MPa exceeded effective yield strength {:.2} MPa at {:.2} K",
                segment_name,
                thickness.value(),
                material.name(),
                failure_mode,
                total_stress.value() / 1.0e6,
                eff_yield.value() / 1.0e6,
                t_val
            ),
        });
    }

    Ok(())
}

pub fn check_material_thermal_structural_limits(
    segment_name: &str,
    material: &AerospaceMaterial,
    thickness: Length,
    segment_temperature: Temperature,
) -> RocketDomainResult<()> {
    check_material_thermal_structural_limits_with_stress(
        segment_name,
        material,
        thickness,
        segment_temperature,
        None,
        None,
        None,
    )
}

pub fn check_material_record_thermal_structural_limits(
    segment_name: &str,
    record: &MaterialRecord,
    thickness: Length,
    segment_temperature: Temperature,
) -> RocketDomainResult<()> {
    let custom_exp = record
        .ablative_properties()
        .and_then(|p| p.thermal_softening_exponent());
    check_material_thermal_structural_limits_with_stress(
        segment_name,
        record.material(),
        thickness,
        segment_temperature,
        None,
        None,
        custom_exp,
    )
}