use crate::ffi::VisualComponent;
use rocketcon_core::domain::ComponentKind;
use rocketcon_sim::VehicleVisualComponent;

impl From<&VehicleVisualComponent> for VisualComponent {
    fn from(source: &VehicleVisualComponent) -> Self {
        Self {
            instance_id: source.instance_id.clone(),
            name: source.name.clone(),
            kind: kind_code(source.kind),
            stage_index: source.stage_index,
            offset_x_m: source.mount_offset_m[0],
            offset_y_m: source.mount_offset_m[1],
            offset_z_m: source.mount_offset_m[2],
            length_m: source.length_m,
            diameter_m: source.diameter_m,
            has_min_throttle: source.min_throttle_fraction.is_some(),
            min_throttle_fraction: source.min_throttle_fraction.unwrap_or_default(),
        }
    }
}

fn kind_code(kind: ComponentKind) -> u8 {
    match kind {
        ComponentKind::Engine => 0,
        ComponentKind::PropellantTank => 1,
        ComponentKind::Battery => 2,
        ComponentKind::SolarPanel => 3,
        ComponentKind::Cpu => 4,
        ComponentKind::ReactionControlThruster => 5,
        ComponentKind::ReactionWheel => 6,
        ComponentKind::Rtg => 7,
        ComponentKind::NuclearReactor => 8,
        ComponentKind::Radiator => 9,
        ComponentKind::PayloadFairing => 10,
        ComponentKind::PayloadDispenser => 11,
        ComponentKind::Hull => 12,
        ComponentKind::HeatShield => 13,
    }
}
