use crate::error::{RocketDomainError, RocketDomainResult};
use astronomicon_core::domain::validation::validate_finite;
use astronomicon_core::units::Vector3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehicleComponentEntry {
    id: Uuid,
    vehicle_id: Uuid,
    component_id: Uuid,
    stage_index: u32,
    instance_label: Option<String>,
    mount_offset: Vector3,
    actuation_axis: Option<Vector3>,
}

impl VehicleComponentEntry {
    pub fn new(
        id: Uuid,
        vehicle_id: Uuid,
        component_id: Uuid,
        stage_index: u32,
        instance_label: Option<String>,
        mount_offset: Vector3,
        actuation_axis: Option<Vector3>,
    ) -> RocketDomainResult<Self> {
        validate_finite(mount_offset.0, "mount_offset_x")?;
        validate_finite(mount_offset.1, "mount_offset_y")?;
        validate_finite(mount_offset.2, "mount_offset_z")?;

        let normalized_actuation_axis = match actuation_axis {
            Some(axis) => {
                validate_finite(axis.0, "actuation_axis_x")?;
                validate_finite(axis.1, "actuation_axis_y")?;
                validate_finite(axis.2, "actuation_axis_z")?;
                if axis.magnitude() < 1e-12 {
                    return Err(RocketDomainError::InvalidInvariant {
                        field: "actuation_axis".to_string(),
                        reason: "actuation axis magnitude cannot be zero".to_string(),
                    });
                }
                Some(axis.normalized())
            }
            None => None,
        };

        Ok(Self {
            id,
            vehicle_id,
            component_id,
            stage_index,
            instance_label,
            mount_offset,
            actuation_axis: normalized_actuation_axis,
        })
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn vehicle_id(&self) -> Uuid {
        self.vehicle_id
    }

    pub fn component_id(&self) -> Uuid {
        self.component_id
    }

    pub fn stage_index(&self) -> u32 {
        self.stage_index
    }

    pub fn instance_label(&self) -> Option<&str> {
        self.instance_label.as_deref()
    }

    pub fn mount_offset(&self) -> Vector3 {
        self.mount_offset
    }

    pub fn actuation_axis(&self) -> Option<Vector3> {
        self.actuation_axis
    }
}