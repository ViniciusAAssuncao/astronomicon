use crate::error::RocketDbResult;
use crate::save::migrations::run_rocketcon_migrations;
use crate::save::paths::{
    list_existing_saves, most_recent_save, save_database_url, save_filename, save_path,
};
use crate::save::template::create_save_copy;
use rocketcon_core::domain::SaveMetadata;
use rocketcon_core::error::RocketDomainError;
use sqlx::SqlitePool;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub async fn resolve_current_save_pool() -> RocketDbResult<SqlitePool> {
    let saves = list_existing_saves().await?;
    let (resolved_path, new_save_info) = if let Some(recent) = most_recent_save(&saves) {
        (recent.clone(), None)
    } else {
        let uuid = Uuid::new_v4();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| RocketDomainError::InvalidInvariant {
                field: "created_at_unix_seconds".to_string(),
                reason: "system clock is before UNIX epoch".to_string(),
            })?;
        let timestamp = now.as_secs() as i64;
        let filename = save_filename(timestamp);
        let destination = save_path(&filename);
        create_save_copy(&destination).await?;
        (destination, Some((uuid, timestamp)))
    };

    if !resolved_path.exists() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "resolved_path".to_string(),
            reason: format!(
                "save path '{}' does not exist on disk",
                resolved_path.display()
            ),
        }
        .into());
    }

    let filename = resolved_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| RocketDomainError::InvalidInvariant {
            field: "resolved_path".to_string(),
            reason: "save path has invalid filename".to_string(),
        })?;

    let url = save_database_url(filename);
    let pool = astronomicon_db::connection::open_pool(&url).await?;

    run_rocketcon_migrations(&pool).await?;

    if let Some((uuid, timestamp)) = new_save_info {
        let metadata = SaveMetadata::new(
            uuid,
            timestamp,
            astronomicon_db::connection::DATABASE_URL.to_string(),
        )?;
        crate::repositories::save_metadata::insert(&pool, &metadata).await?;
    }

    Ok(pool)
}