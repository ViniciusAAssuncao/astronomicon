use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComponentKind {
    Engine,
    PropellantTank,
    Battery,
    SolarPanel,
    Cpu,
    ReactionControlThruster,
    ReactionWheel,
    Rtg,
    NuclearReactor,
    Radiator,
    PayloadFairing,
    PayloadDispenser,
    Hull,
}