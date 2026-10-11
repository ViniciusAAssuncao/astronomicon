use crate::error::RocketDbResult;
use crate::models::MaterialRow;
use crate::repositories::eav_attributes::{
    fetch_eav_attribute_map, optional_numeric, required_numeric,
};
use astronomicon_core::units::{SpecificEnergy, Temperature};
use rocketcon_core::domain::{
    AblativeMaterialProperties, AerospaceMaterial, MaterialClass, MaterialClassDetails,
    MaterialRecord,
};
use sqlx::SqlitePool;
use uuid::Uuid;

const BASE_QUERY: &str =
    "SELECT id, name, material_class, density_kg_per_m3, specific_heat_capacity_j_per_kg_k, thermal_conductivity_w_per_m_k, thermal_expansion_coefficient_per_k, melting_point_k, max_service_temperature_k, youngs_modulus_pa, base_yield_strength_pa, base_ultimate_tensile_strength_pa, emissivity, solar_absorptivity, manufacturer, manufactured_at_unix_seconds, lore_notes FROM materials";

async fn fetch_material_details(
    pool: &SqlitePool,
    material: &AerospaceMaterial,
) -> RocketDbResult<Option<MaterialClassDetails>> {
    let id = material.id();
    match material.material_class() {
        MaterialClass::AblativeComposite => {
            let attr_map =
                fetch_eav_attribute_map(pool, "material_attributes", "material_id", &id).await?;
            if attr_map.is_empty() {
                return Ok(None);
            }

            let heat_of_ablation_j_per_kg =
                required_numeric(&attr_map, &id, "heat_of_ablation_j_per_kg")?;
            let char_yield_fraction = required_numeric(&attr_map, &id, "char_yield_fraction")?;
            let recession_temperature_onset_k =
                required_numeric(&attr_map, &id, "recession_temperature_onset_k")?;
            let pyrolysis_gas_blowing_coefficient =
                required_numeric(&attr_map, &id, "pyrolysis_gas_blowing_coefficient")?;
            let thermal_softening_exponent =
                optional_numeric(&attr_map, &id, "thermal_softening_exponent")?;

            let props = AblativeMaterialProperties::new(
                SpecificEnergy::new(heat_of_ablation_j_per_kg),
                char_yield_fraction,
                Temperature::new(recession_temperature_onset_k),
                pyrolysis_gas_blowing_coefficient,
                thermal_softening_exponent,
            )?;

            Ok(Some(MaterialClassDetails::Ablative(props)))
        }
        _ => Ok(None),
    }
}

pub async fn get_by_id(pool: &SqlitePool, id: &Uuid) -> RocketDbResult<Option<MaterialRecord>> {
    let query = format!("{BASE_QUERY} WHERE id = ?");
    let row = sqlx::query_as::<_, MaterialRow>(&query)
        .bind(id.to_string())
        .fetch_optional(pool)
        .await?;

    let material_row = match row {
        Some(r) => r,
        None => return Ok(None),
    };

    let material = AerospaceMaterial::try_from(material_row)?;
    let details = fetch_material_details(pool, &material).await?;

    Ok(Some(MaterialRecord::new(material, details)))
}

pub async fn list_all(pool: &SqlitePool) -> RocketDbResult<Vec<MaterialRecord>> {
    let query = format!("{BASE_QUERY} ORDER BY name ASC");
    let rows = sqlx::query_as::<_, MaterialRow>(&query)
        .fetch_all(pool)
        .await?;

    let mut records = Vec::with_capacity(rows.len());
    for row in rows {
        let material = AerospaceMaterial::try_from(row)?;
        let details = fetch_material_details(pool, &material).await?;
        records.push(MaterialRecord::new(material, details));
    }

    Ok(records)
}

pub async fn list_by_class(
    pool: &SqlitePool,
    material_class: MaterialClass,
) -> RocketDbResult<Vec<MaterialRecord>> {
    let query = format!("{BASE_QUERY} WHERE material_class = ? ORDER BY name ASC");
    let rows = sqlx::query_as::<_, MaterialRow>(&query)
        .bind(material_class.as_str())
        .fetch_all(pool)
        .await?;

    let mut records = Vec::with_capacity(rows.len());
    for row in rows {
        let material = AerospaceMaterial::try_from(row)?;
        let details = fetch_material_details(pool, &material).await?;
        records.push(MaterialRecord::new(material, details));
    }

    Ok(records)
}
