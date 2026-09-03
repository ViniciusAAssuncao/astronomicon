use crate::error::{RocketDbError, RocketDbResult};
use rocketcon_core::error::RocketDomainError;
use sqlx::{FromRow, SqlitePool};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum AttributeValue {
    Numeric(f64),
    Text(String),
}

#[derive(Debug, Clone, FromRow)]
struct AttributeRow {
    attribute_key: String,
    numeric_value: Option<f64>,
    text_value: Option<String>,
}

pub async fn fetch_eav_attribute_map(
    pool: &SqlitePool,
    table_name: &str,
    id_column: &str,
    entity_id: &Uuid,
) -> RocketDbResult<HashMap<String, AttributeValue>> {
    let query = format!(
        "SELECT attribute_key, numeric_value, text_value FROM {} WHERE {} = ?",
        table_name, id_column
    );
    let rows = sqlx::query_as::<_, AttributeRow>(&query)
        .bind(entity_id.to_string())
        .fetch_all(pool)
        .await?;

    let mut map = HashMap::with_capacity(rows.len());
    for row in rows {
        let val = if let Some(n) = row.numeric_value {
            AttributeValue::Numeric(n)
        } else if let Some(t) = row.text_value {
            AttributeValue::Text(t)
        } else {
            return Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                field: row.attribute_key,
                reason: format!(
                    "attribute for '{}' in table '{}' had both numeric and text value null",
                    entity_id, table_name
                ),
            }));
        };
        map.insert(row.attribute_key, val);
    }

    Ok(map)
}

pub fn required_numeric(
    map: &HashMap<String, AttributeValue>,
    entity_id: &Uuid,
    key: &str,
) -> RocketDbResult<f64> {
    match map.get(key) {
        Some(AttributeValue::Numeric(val)) => Ok(*val),
        Some(AttributeValue::Text(_)) => {
            Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                field: key.to_string(),
                reason: format!(
                    "expected numeric attribute '{}' for entity '{}', found text",
                    key, entity_id
                ),
            }))
        }
        None => Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
            field: key.to_string(),
            reason: format!(
                "missing required numeric attribute '{}' for entity '{}'",
                key, entity_id
            ),
        })),
    }
}

pub fn optional_numeric(
    map: &HashMap<String, AttributeValue>,
    entity_id: &Uuid,
    key: &str,
) -> RocketDbResult<Option<f64>> {
    match map.get(key) {
        Some(AttributeValue::Numeric(val)) => Ok(Some(*val)),
        Some(AttributeValue::Text(_)) => {
            Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                field: key.to_string(),
                reason: format!(
                    "expected numeric attribute '{}' for entity '{}', found text",
                    key, entity_id
                ),
            }))
        }
        None => Ok(None),
    }
}

pub fn required_text<'a>(
    map: &'a HashMap<String, AttributeValue>,
    entity_id: &Uuid,
    key: &str,
) -> RocketDbResult<&'a str> {
    match map.get(key) {
        Some(AttributeValue::Text(val)) => Ok(val.as_str()),
        Some(AttributeValue::Numeric(_)) => {
            Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                field: key.to_string(),
                reason: format!(
                    "expected text attribute '{}' for entity '{}', found numeric",
                    key, entity_id
                ),
            }))
        }
        None => Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
            field: key.to_string(),
            reason: format!(
                "missing required text attribute '{}' for entity '{}'",
                key, entity_id
            ),
        })),
    }
}

pub fn optional_text<'a>(
    map: &'a HashMap<String, AttributeValue>,
    entity_id: &Uuid,
    key: &str,
) -> RocketDbResult<Option<&'a str>> {
    match map.get(key) {
        Some(AttributeValue::Text(val)) => Ok(Some(val.as_str())),
        Some(AttributeValue::Numeric(_)) => {
            Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                field: key.to_string(),
                reason: format!(
                    "expected text attribute '{}' for entity '{}', found numeric",
                    key, entity_id
                ),
            }))
        }
        None => Ok(None),
    }
}

pub fn required_bool(
    map: &HashMap<String, AttributeValue>,
    entity_id: &Uuid,
    key: &str,
) -> RocketDbResult<bool> {
    let num = required_numeric(map, entity_id, key)?;
    Ok(num == 1.0)
}

pub fn required_uuid(
    map: &HashMap<String, AttributeValue>,
    entity_id: &Uuid,
    key: &str,
) -> RocketDbResult<Uuid> {
    let text = required_text(map, entity_id, key)?;
    Uuid::parse_str(text).map_err(|e| {
        RocketDbError::Domain(RocketDomainError::InvalidInvariant {
            field: key.to_string(),
            reason: format!(
                "invalid uuid for attribute '{}' on entity '{}': {}",
                key, entity_id, e
            ),
        })
    })
}

pub fn optional_uuid(
    map: &HashMap<String, AttributeValue>,
    entity_id: &Uuid,
    key: &str,
) -> RocketDbResult<Option<Uuid>> {
    match optional_text(map, entity_id, key)? {
        Some(text) => Uuid::parse_str(text).map(Some).map_err(|e| {
            RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                field: key.to_string(),
                reason: format!(
                    "invalid uuid for attribute '{}' on entity '{}': {}",
                    key, entity_id, e
                ),
            })
        }),
        None => Ok(None),
    }
}
