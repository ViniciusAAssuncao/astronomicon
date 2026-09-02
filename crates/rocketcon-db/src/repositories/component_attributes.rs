use crate::error::{RocketDbError, RocketDbResult};
use rocketcon_core::error::RocketDomainError;
use sqlx::{FromRow, SqlitePool};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum ComponentAttributeValue {
    Numeric(f64),
    Text(String),
}

#[derive(Debug, Clone, FromRow)]
struct ComponentAttributeRow {
    attribute_key: String,
    numeric_value: Option<f64>,
    text_value: Option<String>,
}

pub async fn fetch_attribute_map(
    pool: &SqlitePool,
    component_id: &Uuid,
) -> RocketDbResult<HashMap<String, ComponentAttributeValue>> {
    let rows = sqlx::query_as::<_, ComponentAttributeRow>(
        "SELECT attribute_key, numeric_value, text_value FROM component_attributes WHERE component_id = ?",
    )
    .bind(component_id.to_string())
    .fetch_all(pool)
    .await?;

    let mut map = HashMap::with_capacity(rows.len());
    for row in rows {
        let val = if let Some(n) = row.numeric_value {
            ComponentAttributeValue::Numeric(n)
        } else if let Some(t) = row.text_value {
            ComponentAttributeValue::Text(t)
        } else {
            return Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                field: row.attribute_key,
                reason: format!(
                    "component attribute for '{}' had both numeric and text value null",
                    component_id
                ),
            }));
        };
        map.insert(row.attribute_key, val);
    }

    Ok(map)
}

pub fn required_numeric(
    map: &HashMap<String, ComponentAttributeValue>,
    component_id: &Uuid,
    key: &str,
) -> RocketDbResult<f64> {
    match map.get(key) {
        Some(ComponentAttributeValue::Numeric(val)) => Ok(*val),
        Some(ComponentAttributeValue::Text(_)) => {
            Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                field: key.to_string(),
                reason: format!(
                    "expected numeric attribute '{}' for component '{}', found text",
                    key, component_id
                ),
            }))
        }
        None => Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
            field: key.to_string(),
            reason: format!(
                "missing required numeric attribute '{}' for component '{}'",
                key, component_id
            ),
        })),
    }
}

pub fn optional_numeric(
    map: &HashMap<String, ComponentAttributeValue>,
    component_id: &Uuid,
    key: &str,
) -> RocketDbResult<Option<f64>> {
    match map.get(key) {
        Some(ComponentAttributeValue::Numeric(val)) => Ok(Some(*val)),
        Some(ComponentAttributeValue::Text(_)) => {
            Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                field: key.to_string(),
                reason: format!(
                    "expected numeric attribute '{}' for component '{}', found text",
                    key, component_id
                ),
            }))
        }
        None => Ok(None),
    }
}

pub fn required_text<'a>(
    map: &'a HashMap<String, ComponentAttributeValue>,
    component_id: &Uuid,
    key: &str,
) -> RocketDbResult<&'a str> {
    match map.get(key) {
        Some(ComponentAttributeValue::Text(val)) => Ok(val.as_str()),
        Some(ComponentAttributeValue::Numeric(_)) => {
            Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                field: key.to_string(),
                reason: format!(
                    "expected text attribute '{}' for component '{}', found numeric",
                    key, component_id
                ),
            }))
        }
        None => Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
            field: key.to_string(),
            reason: format!(
                "missing required text attribute '{}' for component '{}'",
                key, component_id
            ),
        })),
    }
}

pub fn optional_text<'a>(
    map: &'a HashMap<String, ComponentAttributeValue>,
    component_id: &Uuid,
    key: &str,
) -> RocketDbResult<Option<&'a str>> {
    match map.get(key) {
        Some(ComponentAttributeValue::Text(val)) => Ok(Some(val.as_str())),
        Some(ComponentAttributeValue::Numeric(_)) => {
            Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                field: key.to_string(),
                reason: format!(
                    "expected text attribute '{}' for component '{}', found numeric",
                    key, component_id
                ),
            }))
        }
        None => Ok(None),
    }
}

pub fn required_bool(
    map: &HashMap<String, ComponentAttributeValue>,
    component_id: &Uuid,
    key: &str,
) -> RocketDbResult<bool> {
    let num = required_numeric(map, component_id, key)?;
    Ok(num == 1.0)
}

pub fn required_uuid(
    map: &HashMap<String, ComponentAttributeValue>,
    component_id: &Uuid,
    key: &str,
) -> RocketDbResult<Uuid> {
    let text = required_text(map, component_id, key)?;
    Uuid::parse_str(text).map_err(|e| {
        RocketDbError::Domain(RocketDomainError::InvalidInvariant {
            field: key.to_string(),
            reason: format!(
                "invalid uuid for attribute '{}' on component '{}': {}",
                key, component_id, e
            ),
        })
    })
}

pub fn optional_uuid(
    map: &HashMap<String, ComponentAttributeValue>,
    component_id: &Uuid,
    key: &str,
) -> RocketDbResult<Option<Uuid>> {
    match optional_text(map, component_id, key)? {
        Some(text) => Uuid::parse_str(text).map(Some).map_err(|e| {
            RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                field: key.to_string(),
                reason: format!(
                    "invalid uuid for attribute '{}' on component '{}': {}",
                    key, component_id, e
                ),
            })
        }),
        None => Ok(None),
    }
}