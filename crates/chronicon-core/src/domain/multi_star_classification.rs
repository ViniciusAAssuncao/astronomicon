use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MultiStarOrbitType {
    SingleStar,
    PType,
    SType,
    NonStellarParent,
}

impl MultiStarOrbitType {
    pub fn is_single_star(&self) -> bool {
        matches!(self, Self::SingleStar)
    }

    pub fn is_p_type(&self) -> bool {
        matches!(self, Self::PType)
    }

    pub fn is_s_type(&self) -> bool {
        matches!(self, Self::SType)
    }

    pub fn is_multi_star(&self) -> bool {
        matches!(self, Self::PType | Self::SType)
    }
}