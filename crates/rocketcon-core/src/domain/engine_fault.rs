use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EngineFaultKind {
    IgnitionFailure,
    PrimingFailure,
    FlameOut,
    TurbopumpFailure,
    ChamberBreach,
    PropellantStarvation,
    GimbalActuatorFailure,
    ValveFailure,
    Overpressure,
}