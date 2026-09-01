use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehicleComponentEntry {
    id: Uuid,
    vehicle_id: Uuid,
    component_id: Uuid,
    instance_label: Option<String>,
}

impl VehicleComponentEntry {
    pub fn new(
        id: Uuid,
        vehicle_id: Uuid,
        component_id: Uuid,
        instance_label: Option<String>,
    ) -> Self {
        Self {
            id,
            vehicle_id,
            component_id,
            instance_label,
        }
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

    pub fn instance_label(&self) -> Option<&str> {
        self.instance_label.as_deref()
    }
}