use crate::error::{ RocketError, RocketResult };
use astronomicon_core::units::{ Duration, Mass, Position, Vector3, VelocityVector };
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::{
    ComponentDetails,
    ComponentPayloadState,
    PayloadDeploymentEvent,
    VehiclePhysicalState,
};
use rocketcon_db::repositories::{
    payload_state as payload_state_repository,
    vehicle as vehicle_repository,
    vehicle_physical_state as vehicle_physical_state_repository,
};
use serde::{ Deserialize, Serialize };
use std::ops::Deref;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PayloadDeploymentResult {
    pub event: PayloadDeploymentEvent,
    pub child_physical_state: Option<VehiclePhysicalState>,
}

impl PayloadDeploymentResult {
    pub fn new(
        event: PayloadDeploymentEvent,
        child_physical_state: Option<VehiclePhysicalState>
    ) -> Self {
        Self {
            event,
            child_physical_state,
        }
    }

    pub fn event(&self) -> &PayloadDeploymentEvent {
        &self.event
    }

    pub fn child_physical_state(&self) -> Option<&VehiclePhysicalState> {
        self.child_physical_state.as_ref()
    }
}

impl Deref for PayloadDeploymentResult {
    type Target = PayloadDeploymentEvent;

    fn deref(&self) -> &Self::Target {
        &self.event
    }
}

pub async fn deploy_payload(
    pool: &SqlitePool,
    mother_vehicle_id: Uuid,
    vehicle_component_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration
) -> RocketResult<PayloadDeploymentResult> {
    let mother_state = vehicle_physical_state_repository
        ::get_by_vehicle_id(pool, &mother_vehicle_id).await?
        .ok_or_else(|| {
            RocketError::Generic(
                format!("physical state for vehicle '{}' not found", mother_vehicle_id)
            )
        })?;

    let components = vehicle_repository::list_components_for_vehicle(
        pool,
        &mother_vehicle_id
    ).await?;

    let (entry, record) = components
        .iter()
        .find(|(e, _)| e.id() == vehicle_component_id)
        .ok_or_else(|| {
            RocketError::Generic(
                format!(
                    "vehicle component '{}' not found on vehicle '{}'",
                    vehicle_component_id,
                    mother_vehicle_id
                )
            )
        })?;

    let payload_spec = match record.details() {
        ComponentDetails::Payload(spec) => spec,
        _ => {
            return Err(
                RocketError::Generic(
                    format!("component '{}' is not a payload dispenser or fairing", vehicle_component_id)
                )
            );
        }
    };

    let already_deployed = payload_state_repository
        ::is_deployed(pool, &vehicle_component_id).await?
        .unwrap_or(false);

    if already_deployed {
        return Err(
            RocketError::Generic(
                format!("payload for component '{}' is already deployed", vehicle_component_id)
            )
        );
    }

    let ejected_mass = if let Some(contained_id) = payload_spec.contained_vehicle_id() {
        crate::aeroespacial::resolve_vehicle_real_mass(
            pool,
            contained_id,
            universe_epoch,
            at_epoch
        ).await?
    } else if let Some(cargo_mass) = payload_spec.generic_cargo_mass() {
        cargo_mass
    } else {
        Mass::new(0.0)
    };

    let mount_offset = entry.mount_offset();
    let rotated_mount_offset = mother_state.orientation().rotate_vector(mount_offset);
    let child_position = Position::from_components(
        mother_state.position().raw().0 + rotated_mount_offset.0,
        mother_state.position().raw().1 + rotated_mount_offset.1,
        mother_state.position().raw().2 + rotated_mount_offset.2
    );

    let local_axis = entry.actuation_axis().unwrap_or(Vector3::new(0.0, 0.0, 1.0)).normalized();
    let global_directional_vector = mother_state
        .orientation()
        .rotate_vector(local_axis)
        .normalized();

    let sep_speed = payload_spec.separation_velocity().value();
    let child_velocity = VelocityVector::from_components(
        mother_state.velocity().raw().0 + global_directional_vector.0 * sep_speed,
        mother_state.velocity().raw().1 + global_directional_vector.1 * sep_speed,
        mother_state.velocity().raw().2 + global_directional_vector.2 * sep_speed
    );

    let payload_state = ComponentPayloadState::new(
        vehicle_component_id,
        true,
        universe_epoch,
        at_epoch
    )?;
    payload_state_repository::upsert(pool, &payload_state).await?;

    let child_physical_state = if let Some(contained_id) = payload_spec.contained_vehicle_id() {
        let state = VehiclePhysicalState::new(
            contained_id,
            child_position,
            child_velocity,
            mother_state.orientation(),
            mother_state.angular_velocity(),
            mother_state.reference_body_id(),
            universe_epoch,
            at_epoch
        )?;
        vehicle_physical_state_repository::upsert(pool, &state).await?;
        Some(state)
    } else {
        None
    };

    let event = PayloadDeploymentEvent::new_with_vehicle(
        vehicle_component_id,
        ejected_mass,
        payload_spec.contained_vehicle_id()
    )?;

    Ok(PayloadDeploymentResult::new(event, child_physical_state))
}
