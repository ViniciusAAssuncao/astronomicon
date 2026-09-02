use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VehicleKind {
    Rocket,
    Spacecraft,
    Probe,
    Rover,
    Satellite,
}
