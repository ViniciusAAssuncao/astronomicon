use super::orbit_preview::{OrbitPreview, preview_from_pool};
use crate::{RocketError, RocketResult};
use astronomicon_core::units::Duration;
use astronomicon_db::SqlitePool;
use std::path::Path;
use uuid::Uuid;

pub struct ForecastReader {
    pool: SqlitePool,
    vehicle_id: Uuid,
    universe_epoch: Duration,
}

impl ForecastReader {
    pub async fn open(path: impl AsRef<Path>, vehicle_id: Uuid) -> RocketResult<Self> {
        let path = path.as_ref();
        if !path.is_file() {
            return Err(RocketError::Generic(format!(
                "forecast save not found: {}",
                path.display(),
            )));
        }
        let pool = astronomicon_db::connection::open_read_only_pool_path(path).await?;
        let universe_epoch = rocketcon_app::universe::resolve_universe_epoch(&pool).await?;
        Ok(Self {
            pool,
            vehicle_id,
            universe_epoch,
        })
    }

    pub async fn preview(&self, under_thrust: bool) -> RocketResult<OrbitPreview> {
        preview_from_pool(
            &self.pool,
            self.vehicle_id,
            self.universe_epoch,
            None,
            under_thrust,
        )
        .await
    }

    pub async fn close(self) {
        self.pool.close().await;
    }
}
