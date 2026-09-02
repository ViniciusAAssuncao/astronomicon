use crate::domain::{
    ComponentDetails, ComponentOperationalState, ComponentRecord, EnergyReservoirState,
    VehicleComponentEntry,
};
use crate::error::{RocketDomainError, RocketDomainResult};
use crate::math::{resolve_mass_properties, MassProperties};
use astronomicon_core::domain::validation::validate_non_negative_finite;
use astronomicon_core::units::{Duration, Energy, Mass};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehicleSnapshot {
    pub vehicle_id: Uuid,
    pub active_stages: Vec<u32>,
    pub total_dry_mass: Mass,
    pub total_battery_capacity: Energy,
    pub total_stored_energy: Energy,
    pub mass_properties: MassProperties,
    pub engine_operational_states: HashMap<Uuid, ComponentOperationalState>,
    pub captured_universe_epoch: Duration,
    pub captured_at_epoch: Duration,
}

impl VehicleSnapshot {
    pub fn new(
        vehicle_id: Uuid,
        active_stages: Vec<u32>,
        total_dry_mass: Mass,
        total_battery_capacity: Energy,
        total_stored_energy: Energy,
        mass_properties: MassProperties,
        engine_operational_states: HashMap<Uuid, ComponentOperationalState>,
        captured_universe_epoch: Duration,
        captured_at_epoch: Duration,
    ) -> RocketDomainResult<Self> {
        if active_stages.is_empty() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "active_stages".to_string(),
                reason: "active stages list cannot be empty".to_string(),
            });
        }

        if !captured_universe_epoch.value().is_finite() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "captured_universe_epoch".to_string(),
                reason: "value must be finite".to_string(),
            });
        }

        if !captured_at_epoch.value().is_finite() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "captured_at_epoch".to_string(),
                reason: "value must be finite".to_string(),
            });
        }

        validate_non_negative_finite(total_dry_mass.value(), "total_dry_mass")?;
        validate_non_negative_finite(
            total_battery_capacity.value(),
            "total_battery_capacity",
        )?;
        validate_non_negative_finite(total_stored_energy.value(), "total_stored_energy")?;

        Ok(Self {
            vehicle_id,
            active_stages,
            total_dry_mass,
            total_battery_capacity,
            total_stored_energy,
            mass_properties,
            engine_operational_states,
            captured_universe_epoch,
            captured_at_epoch,
        })
    }

    pub fn from_components(
        vehicle_id: Uuid,
        components: &[(VehicleComponentEntry, ComponentRecord)],
        active_stages: Vec<u32>,
        reservoir_states: &HashMap<Uuid, EnergyReservoirState>,
        operational_states: &HashMap<Uuid, ComponentOperationalState>,
        propellant_load_fraction: f64,
        captured_universe_epoch: Duration,
        captured_at_epoch: Duration,
    ) -> RocketDomainResult<Self> {
        if active_stages.is_empty() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "active_stages".to_string(),
                reason: "active stages list cannot be empty".to_string(),
            });
        }

        let active_entries: Vec<(VehicleComponentEntry, ComponentRecord)> = components
            .iter()
            .filter(|(entry, _)| active_stages.contains(&entry.stage_index()))
            .cloned()
            .collect();

        let mut total_dry_mass_val = 0.0;
        let mut total_battery_capacity_val = 0.0;
        let mut total_stored_energy_val = 0.0;
        let mut engine_states = HashMap::new();

        for (entry, record) in &active_entries {
            total_dry_mass_val += record.component().dry_mass().value();

            match record.details() {
                ComponentDetails::Battery(battery) => {
                    let cap = battery.capacity().value();
                    total_battery_capacity_val += cap;
                    let stored = match reservoir_states.get(&entry.id()) {
                        Some(res) => res.stored_energy().value().min(cap),
                        None => cap,
                    };
                    total_stored_energy_val += stored;
                }
                ComponentDetails::Engine(_) => {
                    if let Some(op_state) = operational_states.get(&entry.id()) {
                        engine_states.insert(entry.id(), *op_state);
                    }
                }
                _ => {}
            }
        }

        let mass_props =
            resolve_mass_properties(components, &active_stages, propellant_load_fraction);

        Self::new(
            vehicle_id,
            active_stages,
            Mass::new(total_dry_mass_val),
            Energy::new(total_battery_capacity_val),
            Energy::new(total_stored_energy_val),
            mass_props,
            engine_states,
            captured_universe_epoch,
            captured_at_epoch,
        )
    }

    pub fn vehicle_id(&self) -> Uuid {
        self.vehicle_id
    }

    pub fn active_stages(&self) -> &[u32] {
        &self.active_stages
    }

    pub fn total_dry_mass(&self) -> Mass {
        self.total_dry_mass
    }

    pub fn total_battery_capacity(&self) -> Energy {
        self.total_battery_capacity
    }

    pub fn battery_capacity(&self) -> Energy {
        self.total_battery_capacity
    }

    pub fn total_stored_energy(&self) -> Energy {
        self.total_stored_energy
    }

    pub fn stored_energy(&self) -> Energy {
        self.total_stored_energy
    }

    pub fn mass_properties(&self) -> &MassProperties {
        &self.mass_properties
    }

    pub fn engine_operational_states(&self) -> &HashMap<Uuid, ComponentOperationalState> {
        &self.engine_operational_states
    }

    pub fn active_engine_operational_states(&self) -> &HashMap<Uuid, ComponentOperationalState> {
        &self.engine_operational_states
    }

    pub fn captured_universe_epoch(&self) -> Duration {
        self.captured_universe_epoch
    }

    pub fn captured_at_epoch(&self) -> Duration {
        self.captured_at_epoch
    }

    pub fn captured_total_epoch(&self) -> Duration {
        self.captured_universe_epoch + self.captured_at_epoch
    }

    pub fn is_stage_active(&self, stage: u32) -> bool {
        self.active_stages.contains(&stage)
    }

    pub fn state_of_charge_fraction(&self) -> f64 {
        let cap = self.total_battery_capacity.value();
        if cap > 0.0 && cap.is_finite() {
            (self.total_stored_energy.value() / cap).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    pub fn is_stale(&self, current_total_epoch: Duration, max_epoch_delta: Duration) -> bool {
        let epoch_diff =
            (current_total_epoch.value() - self.captured_total_epoch().value()).abs();
        epoch_diff > max_epoch_delta.value()
    }
}