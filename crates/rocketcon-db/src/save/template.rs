use crate::error::{RocketDbError, RocketDbResult};
use rocketcon_core::error::RocketDomainError;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, SqliteConnection};
use std::path::Path;
use std::str::FromStr;

pub async fn create_save_copy(destination: &Path) -> RocketDbResult<()> {
    let template_url = astronomicon_db::connection::DATABASE_URL;
    let template_path_str = template_url.strip_prefix("sqlite://").unwrap_or(template_url);

    if !Path::new(template_path_str).exists() {
        return Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
            field: "source_template_path".to_string(),
            reason: "astronomicon database template must exist before creating save".to_string(),
        }));
    }

    let options = SqliteConnectOptions::from_str(template_url)?.read_only(true);
    let mut conn = SqliteConnection::connect_with(&options).await?;

    let dest_str = destination.to_str().ok_or_else(|| {
        RocketDbError::Domain(RocketDomainError::InvalidInvariant {
            field: "destination".to_string(),
            reason: "destination path is not valid UTF-8".to_string(),
        })
    })?;

    sqlx::query("VACUUM INTO ?")
        .bind(dest_str)
        .execute(&mut conn)
        .await?;

    Ok(())
}