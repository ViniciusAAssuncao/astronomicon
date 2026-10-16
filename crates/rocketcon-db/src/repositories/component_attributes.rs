use crate::error::RocketDbResult;
use sqlx::SqlitePool;
use std::collections::HashMap;
use uuid::Uuid;

pub use super::eav_attributes::{
    AttributeValue as ComponentAttributeValue, optional_numeric, optional_text, optional_uuid,
    required_bool, required_numeric, required_text, required_uuid,
};

pub async fn fetch_attribute_map(
    pool: &SqlitePool,
    component_id: &Uuid,
) -> RocketDbResult<HashMap<String, ComponentAttributeValue>> {
    super::eav_attributes::fetch_eav_attribute_map(
        pool,
        "component_attributes",
        "component_id",
        component_id,
    )
    .await
}

pub async fn fetch_material_ids_for_vehicle(
    pool: &SqlitePool,
    vehicle_id: &Uuid,
) -> RocketDbResult<HashMap<Uuid, Uuid>> {
    let rows = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT DISTINCT attributes.component_id, attributes.text_value \
         FROM vehicle_components AS installed \
         JOIN component_attributes AS attributes ON attributes.component_id = installed.component_id \
         WHERE installed.vehicle_id = ? AND attributes.attribute_key = 'material_id'",
    )
    .bind(vehicle_id.to_string())
    .fetch_all(pool)
    .await?;

    let mut material_ids = HashMap::with_capacity(rows.len());
    for (component_id, value) in rows {
        if let (Ok(component_id), Some(material_id)) = (
            Uuid::parse_str(&component_id),
            value.as_deref().and_then(|text| Uuid::parse_str(text).ok()),
        ) {
            material_ids.insert(component_id, material_id);
        }
    }
    Ok(material_ids)
}

pub(crate) type RawComponentAttributeRows = Vec<(String, Option<f64>, Option<String>)>;

pub(crate) async fn fetch_attribute_rows_for_vehicle(
    pool: &SqlitePool,
    vehicle_id: &Uuid,
) -> RocketDbResult<HashMap<String, RawComponentAttributeRows>> {
    let rows = sqlx::query_as::<_, (String, String, Option<f64>, Option<String>)>(
        "SELECT DISTINCT attributes.component_id, attributes.attribute_key,          attributes.numeric_value, attributes.text_value          FROM component_attributes AS attributes          JOIN vehicle_components AS installed ON installed.component_id = attributes.component_id          WHERE installed.vehicle_id = ?",
    )
    .bind(vehicle_id.to_string())
    .fetch_all(pool)
    .await?;

    let mut grouped: HashMap<String, RawComponentAttributeRows> = HashMap::new();
    for (component_id, key, numeric, text) in rows {
        grouped
            .entry(component_id)
            .or_default()
            .push((key, numeric, text));
    }
    Ok(grouped)
}
