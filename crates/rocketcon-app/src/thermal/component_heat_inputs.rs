use crate::error::RocketResult;
use crate::power::consumption::resolve_component_consumption;
use crate::power::generation::resolve_component_generation;
use astronomicon_core::units::{Duration, Luminosity};
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::{
    ComponentOperationalState, ComponentRecord, VehicleComponentEntry, VehiclePhysicalState,
    VehicleSnapshot,
};
use rocketcon_core::environment::EnvironmentSnapshot;
use std::collections::HashMap;
use uuid::Uuid;

pub(crate) async fn resolve_vehicle_component_waste_heats(
    pool: &SqlitePool,
    components: &[(VehicleComponentEntry, ComponentRecord)],
    snapshot: &VehicleSnapshot,
    operational_states: &HashMap<Uuid, Option<ComponentOperationalState>>,
    environment: &EnvironmentSnapshot,
    physical_state: &VehiclePhysicalState,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<HashMap<Uuid, Luminosity>> {
    let mut component_waste_heats: HashMap<Uuid, Luminosity> =
        HashMap::with_capacity(components.len());
    for (entry, record) in components {
        if !snapshot.is_stage_active(entry.stage_index()) {
            continue;
        }
        let op_state = operational_states.get(&entry.id()).copied().flatten();
        let gen_contribution = resolve_component_generation(
            pool,
            entry,
            record,
            op_state,
            environment,
            physical_state.position(),
            physical_state.orientation(),
            universe_epoch,
            at_epoch,
        )
        .await?;
        let con_contribution = resolve_component_consumption(entry, record, op_state);
        let total_waste = gen_contribution.waste_heat + con_contribution.waste_heat;
        component_waste_heats.insert(entry.id(), total_waste);
    }
    Ok(component_waste_heats)
}
