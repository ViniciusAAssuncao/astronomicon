use crate::domain::engine_fault::EngineFaultKind;
use crate::domain::engine_specification::EngineSpecification;
use crate::domain::ignition_type::IgnitionType;
use crate::domain::propellant::Propellant;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EngineState {
    Safed,
    Armed,
    Priming,
    IgnitionSequence,
    Startup,
    MainStage,
    Throttling,
    ShutdownSequence,
    PostBurnTailOff,
    Idle,
    Depleted,
    Fault(EngineFaultKind),
}

pub fn is_valid_engine_transition(
    spec: &EngineSpecification,
    fuel: &Propellant,
    oxidizer: Option<&Propellant>,
    from: EngineState,
    to: EngineState,
) -> bool {
    match from {
        EngineState::Safed => matches!(to, EngineState::Armed),
        EngineState::Armed => {
            if spec.requires_priming(fuel, oxidizer) {
                matches!(to, EngineState::Priming)
            } else {
                matches!(to, EngineState::IgnitionSequence)
            }
        }
        EngineState::Priming => match to {
            EngineState::IgnitionSequence => true,
            EngineState::Fault(EngineFaultKind::PrimingFailure) => true,
            _ => false,
        },
        EngineState::IgnitionSequence => match to {
            EngineState::Startup => true,
            EngineState::Fault(EngineFaultKind::IgnitionFailure) => true,
            _ => false,
        },
        EngineState::Startup => match to {
            EngineState::MainStage => true,
            EngineState::Fault(
                EngineFaultKind::FlameOut
                | EngineFaultKind::TurbopumpFailure
                | EngineFaultKind::ChamberBreach,
            ) => true,
            _ => false,
        },
        EngineState::MainStage => match to {
            EngineState::Throttling => spec.is_throttleable(),
            EngineState::ShutdownSequence => spec.ignition_type() == IgnitionType::Restartable,
            EngineState::PostBurnTailOff => true,
            EngineState::Fault(EngineFaultKind::GimbalActuatorFailure) => spec.has_gimbal(),
            EngineState::Fault(_) => true,
            _ => false,
        },
        EngineState::Throttling => match to {
            EngineState::MainStage => true,
            EngineState::Fault(EngineFaultKind::GimbalActuatorFailure) => spec.has_gimbal(),
            EngineState::Fault(_) => true,
            _ => false,
        },
        EngineState::ShutdownSequence => matches!(to, EngineState::PostBurnTailOff),
        EngineState::PostBurnTailOff => match to {
            EngineState::Idle => spec.ignition_type() == IgnitionType::Restartable,
            EngineState::Depleted => true,
            _ => false,
        },
        EngineState::Idle => {
            spec.ignition_type() == IgnitionType::Restartable && matches!(to, EngineState::Armed)
        }
        EngineState::Fault(_) | EngineState::Depleted => matches!(to, EngineState::Safed),
    }
}