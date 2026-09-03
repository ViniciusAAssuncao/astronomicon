use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MaterialClass {
    Metal,
    CompositeLaminate,
    Ceramic,
    AblativeComposite,
    Polymer,
    Exotic,
}

impl MaterialClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Metal => "Metal",
            Self::CompositeLaminate => "CompositeLaminate",
            Self::Ceramic => "Ceramic",
            Self::AblativeComposite => "AblativeComposite",
            Self::Polymer => "Polymer",
            Self::Exotic => "Exotic",
        }
    }
}
