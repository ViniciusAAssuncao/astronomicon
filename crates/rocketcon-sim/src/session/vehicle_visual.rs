use rocketcon_core::domain::{ComponentKind, ComponentRecord, VehicleComponentEntry};

#[derive(Debug, Clone, PartialEq)]
pub struct VehicleVisualComponent {
    pub instance_id: String,
    pub name: String,
    pub kind: ComponentKind,
    pub stage_index: u32,
    pub mount_offset_m: [f64; 3],
    pub length_m: f64,
    pub diameter_m: f64,
}

impl VehicleVisualComponent {
    pub fn from_assembly(entry: &VehicleComponentEntry, record: &ComponentRecord) -> Self {
        let offset = entry.mount_offset();
        let component = record.component();
        Self {
            instance_id: entry.id().to_string(),
            name: entry
                .instance_label()
                .unwrap_or(component.name())
                .to_owned(),
            kind: component.kind(),
            stage_index: entry.stage_index(),
            mount_offset_m: [offset.0, offset.1, offset.2],
            length_m: component.length().value(),
            diameter_m: component.diameter().value(),
        }
    }
}
