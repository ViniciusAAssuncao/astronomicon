use crate::error::RocketDbError;
use astronomicon_core::units::{Density, Pressure, Temperature};
use rocketcon_core::domain::{AerospaceMaterial, MaterialClass};
use rocketcon_core::error::RocketDomainError;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct MaterialRow {
    pub id: String,
    pub name: String,
    pub material_class: String,
    pub density_kg_per_m3: f64,
    pub specific_heat_capacity_j_per_kg_k: f64,
    pub thermal_conductivity_w_per_m_k: f64,
    pub thermal_expansion_coefficient_per_k: f64,
    pub melting_point_k: Option<f64>,
    pub max_service_temperature_k: f64,
    pub youngs_modulus_pa: f64,
    pub base_yield_strength_pa: f64,
    pub base_ultimate_tensile_strength_pa: f64,
    pub emissivity: f64,
    pub solar_absorptivity: f64,
    pub manufacturer: Option<String>,
    pub manufactured_at_unix_seconds: Option<i64>,
    pub lore_notes: Option<String>,
}

impl TryFrom<MaterialRow> for AerospaceMaterial {
    type Error = RocketDbError;

    fn try_from(row: MaterialRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.id)?;
        let material_class = match row.material_class.as_str() {
            "Metal" => MaterialClass::Metal,
            "CompositeLaminate" => MaterialClass::CompositeLaminate,
            "Ceramic" => MaterialClass::Ceramic,
            "AblativeComposite" => MaterialClass::AblativeComposite,
            "Polymer" => MaterialClass::Polymer,
            "Exotic" => MaterialClass::Exotic,
            other => {
                return Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                    field: "material_class".to_string(),
                    reason: format!("unknown material class: {}", other),
                }));
            }
        };

        let material = AerospaceMaterial::builder(
            id,
            row.name,
            material_class,
            Density::new(row.density_kg_per_m3),
            row.specific_heat_capacity_j_per_kg_k,
            row.thermal_conductivity_w_per_m_k,
            row.thermal_expansion_coefficient_per_k,
            Temperature::new(row.max_service_temperature_k),
            Pressure::new(row.youngs_modulus_pa),
            Pressure::new(row.base_yield_strength_pa),
            Pressure::new(row.base_ultimate_tensile_strength_pa),
            row.emissivity,
            row.solar_absorptivity,
        )
        .with_melting_point(row.melting_point_k.map(Temperature::new))
        .with_manufacturer(row.manufacturer)
        .with_manufactured_at_unix_seconds(row.manufactured_at_unix_seconds)
        .with_lore_notes(row.lore_notes)
        .build()?;

        Ok(material)
    }
}
