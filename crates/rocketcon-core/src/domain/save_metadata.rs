use crate::error::{RocketDomainError, RocketDomainResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveMetadata {
    save_uuid: Uuid,
    created_at_unix_seconds: i64,
    source_template_path: String,
}

impl SaveMetadata {
    pub fn new(
        save_uuid: Uuid,
        created_at_unix_seconds: i64,
        source_template_path: String,
    ) -> RocketDomainResult<Self> {
        if save_uuid.is_nil() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "save_uuid".to_string(),
                reason: "cannot be nil".to_string(),
            });
        }

        if created_at_unix_seconds <= 0 {
            return Err(RocketDomainError::InvalidInvariant {
                field: "created_at_unix_seconds".to_string(),
                reason: "must be greater than zero".to_string(),
            });
        }

        if source_template_path.trim().is_empty() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "source_template_path".to_string(),
                reason: "cannot be empty".to_string(),
            });
        }

        Ok(Self {
            save_uuid,
            created_at_unix_seconds,
            source_template_path,
        })
    }

    pub fn save_uuid(&self) -> Uuid {
        self.save_uuid
    }

    pub fn created_at_unix_seconds(&self) -> i64 {
        self.created_at_unix_seconds
    }

    pub fn source_template_path(&self) -> &str {
        &self.source_template_path
    }
}
