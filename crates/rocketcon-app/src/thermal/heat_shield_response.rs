use crate::error::{RocketError, RocketResult};
use astronomicon_core::units::{Duration, HeatFlux, Speed, Temperature};
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::{
    ComponentDetails, ComponentRecord, HeatShieldState, VehicleComponentEntry,
};
use rocketcon_core::error::RocketDomainError;
use rocketcon_core::math::materials::ablation::update_heat_shield_state;
use rocketcon_core::math::thermal_budget::check_material_record_thermal_structural_limits;
use rocketcon_db::repositories::heat_shield_state as heat_shield_state_repository;
use rocketcon_db::repositories::material as material_repository;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveHeatShieldDiagnostic {
    pub vehicle_component_id: Uuid,
    pub component_id: Uuid,
    pub state: HeatShieldState,
    pub recession_rate: Speed,
    pub mass_loss_rate_per_unit_area: f64,
    pub blowing_reduction_factor: f64,
    pub transmitted_heat_flux: HeatFlux,
    pub is_burned_through: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatShieldResponseSummary {
    pub active_shields: Vec<ActiveHeatShieldDiagnostic>,
    pub net_stagnation_heat_flux_to_hull: HeatFlux,
    pub has_active_heat_shield: bool,
    pub has_shield_burnthrough: bool,
}

pub async fn resolve_heat_shield_response(
    pool: &SqlitePool,
    components: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
    raw_stagnation_heat_flux: HeatFlux,
    dt: Duration,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<HeatShieldResponseSummary> {
    let mut active_shields = Vec::new();
    let mut min_transmitted_flux = raw_stagnation_heat_flux.value();
    let mut has_active = false;
    let mut has_burnthrough = false;

    for (entry, record) in components {
        if !active_stages.contains(&entry.stage_index()) {
            continue;
        }

        if let ComponentDetails::HeatShield(spec) = record.details() {
            has_active = true;

            let existing_state = heat_shield_state_repository::get_by_vehicle_component_id(
                pool,
                &entry.id(),
            )
            .await?;

            let current_state = match existing_state {
                Some(s) => s,
                None => HeatShieldState::new(
                    entry.id(),
                    spec.shield_thickness(),
                    Temperature::new(293.15),
                    universe_epoch,
                    at_epoch,
                )?,
            };

            let material_rec = material_repository::get_by_id(pool, &spec.material_id())
                .await?
                .ok_or_else(|| {
                    RocketError::Generic(format!(
                        "material '{}' for heat shield component '{}' not found",
                        spec.material_id(),
                        entry.id()
                    ))
                })?;

            let ablation_res = update_heat_shield_state(
                &current_state,
                &material_rec,
                raw_stagnation_heat_flux,
                dt,
                universe_epoch,
                at_epoch + dt,
            )?;

            heat_shield_state_repository::upsert(pool, &ablation_res.updated_state).await?;

            if ablation_res.is_burned_through {
                has_burnthrough = true;
            }

            let segment_name = entry
                .instance_label()
                .unwrap_or_else(|| record.component().name());

            check_material_record_thermal_structural_limits(
                segment_name,
                &material_rec,
                ablation_res.updated_state.remaining_thickness(),
                ablation_res.updated_state.surface_temperature(),
            )?;

            if ablation_res.is_burned_through {
                return Err(RocketError::Domain(RocketDomainError::StructuralFailure {
                    reason: format!(
                        "Heat shield '{}' completely burned through during atmospheric pass",
                        segment_name
                    ),
                }));
            }

            let trans_flux = ablation_res.transmitted_heat_flux.value();
            if trans_flux < min_transmitted_flux {
                min_transmitted_flux = trans_flux;
            }

            active_shields.push(ActiveHeatShieldDiagnostic {
                vehicle_component_id: entry.id(),
                component_id: entry.component_id(),
                state: ablation_res.updated_state,
                recession_rate: ablation_res.linear_recession_rate,
                mass_loss_rate_per_unit_area: ablation_res.mass_loss_rate_per_unit_area,
                blowing_reduction_factor: ablation_res.blowing_reduction_factor,
                transmitted_heat_flux: ablation_res.transmitted_heat_flux,
                is_burned_through: ablation_res.is_burned_through,
            });
        }
    }

    let net_flux_val = if has_active {
        min_transmitted_flux
    } else {
        raw_stagnation_heat_flux.value()
    };

    Ok(HeatShieldResponseSummary {
        active_shields,
        net_stagnation_heat_flux_to_hull: HeatFlux::new(net_flux_val),
        has_active_heat_shield: has_active,
        has_shield_burnthrough: has_burnthrough,
    })
}