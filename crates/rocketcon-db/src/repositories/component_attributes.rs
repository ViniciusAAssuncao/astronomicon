use crate::error::RocketDbResult;
use sqlx::SqlitePool;
use std::collections::HashMap;
use uuid::Uuid;

pub use super::eav_attributes::{
    optional_numeric, optional_text, optional_uuid, required_bool, required_numeric, required_text,
    required_uuid, AttributeValue as ComponentAttributeValue,
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