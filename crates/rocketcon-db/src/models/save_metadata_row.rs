use crate::error::RocketDbError;
use rocketcon_core::domain::SaveMetadata;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct SaveMetadataRow {
    pub id: i64,
    pub save_uuid: String,
    pub created_at_unix_seconds: i64,
    pub source_template_path: String,
}

impl TryFrom<SaveMetadataRow> for SaveMetadata {
    type Error = RocketDbError;

    fn try_from(row: SaveMetadataRow) -> Result<Self, Self::Error> {
        let uuid = Uuid::parse_str(&row.save_uuid)?;
        let metadata = SaveMetadata::new(
            uuid,
            row.created_at_unix_seconds,
            row.source_template_path,
        )?;
        Ok(metadata)
    }
}