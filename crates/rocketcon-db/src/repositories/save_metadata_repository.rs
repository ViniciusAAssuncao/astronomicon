use crate::error::RocketDbResult;
use crate::models::SaveMetadataRow;
use rocketcon_core::domain::SaveMetadata;
use sqlx::SqlitePool;

pub async fn get(pool: &SqlitePool) -> RocketDbResult<Option<SaveMetadataRow>> {
    let row = sqlx::query_as::<_, SaveMetadataRow>(
        "SELECT id, save_uuid, created_at_unix_seconds, source_template_path FROM save_metadata WHERE id = 1",
    )
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn insert(pool: &SqlitePool, metadata: &SaveMetadata) -> RocketDbResult<()> {
    sqlx::query(
        "INSERT INTO save_metadata (id, save_uuid, created_at_unix_seconds, source_template_path) VALUES (1, ?, ?, ?)",
    )
    .bind(metadata.save_uuid().to_string())
    .bind(metadata.created_at_unix_seconds())
    .bind(metadata.source_template_path())
    .execute(pool)
    .await?;

    Ok(())
}