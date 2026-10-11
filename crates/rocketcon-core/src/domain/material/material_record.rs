use crate::domain::material::ablative_properties::AblativeMaterialProperties;
use crate::domain::material::aerospace_material::AerospaceMaterial;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MaterialClassDetails {
    Ablative(AblativeMaterialProperties),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialRecord {
    material: AerospaceMaterial,
    details: Option<MaterialClassDetails>,
}

impl MaterialRecord {
    pub fn new(material: AerospaceMaterial, details: Option<MaterialClassDetails>) -> Self {
        Self { material, details }
    }

    pub fn material(&self) -> &AerospaceMaterial {
        &self.material
    }

    pub fn details(&self) -> Option<&MaterialClassDetails> {
        self.details.as_ref()
    }

    pub fn ablative_properties(&self) -> Option<&AblativeMaterialProperties> {
        match &self.details {
            Some(MaterialClassDetails::Ablative(props)) => Some(props),
            None => None,
        }
    }
}
