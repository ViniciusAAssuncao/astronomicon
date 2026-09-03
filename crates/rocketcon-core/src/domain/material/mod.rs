pub mod ablative_properties;
pub mod aerospace_material;
pub mod material_class;
pub mod material_record;

pub use ablative_properties::AblativeMaterialProperties;
pub use aerospace_material::{AerospaceMaterial, AerospaceMaterialBuilder};
pub use material_class::MaterialClass;
pub use material_record::{MaterialClassDetails, MaterialRecord};