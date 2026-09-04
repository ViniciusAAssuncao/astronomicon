use crate::error::RocketDomainResult;
use astronomicon_core::domain::validation::validate_positive_finite;
use astronomicon_core::units::Length;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HullSpecification {
    component_id: Uuid,
    material_id: Uuid,
    wall_thickness: Length,
    is_insulated: bool,
}

impl HullSpecification {
    pub fn new(
        component_id: Uuid,
        material_id: Uuid,
        wall_thickness: Length,
        is_insulated: bool,
    ) -> RocketDomainResult<Self> {
        validate_positive_finite(wall_thickness.value(), "wall_thickness")?;

        Ok(Self {
            component_id,
            material_id,
            wall_thickness,
            is_insulated,
        })
    }

    pub fn new_simple(
        component_id: Uuid,
        material_id: Uuid,
        wall_thickness: Length,
    ) -> RocketDomainResult<Self> {
        Self::new(component_id, material_id, wall_thickness, false)
    }

    pub fn component_id(&self) -> Uuid {
        self.component_id
    }

    pub fn material_id(&self) -> Uuid {
        self.material_id
    }

    pub fn wall_thickness(&self) -> Length {
        self.wall_thickness
    }

    pub fn wall_thickness_m(&self) -> f64 {
        self.wall_thickness.value()
    }

    pub fn is_insulated(&self) -> bool {
        self.is_insulated
    }

    pub fn has_mli(&self) -> bool {
        self.is_insulated
    }
}
