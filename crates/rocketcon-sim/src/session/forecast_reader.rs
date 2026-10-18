use super::celestial_atlas::{CelestialAtlas, catalog_at};
use super::celestial_body_atmosphere::append_atmospheric_physics;
use super::celestial_body_climate::append_reference_winds;
use super::celestial_body_details::{CelestialBodyDetails, selected_body_details};
use super::celestial_body_environment::append_reference_environment;
use super::celestial_body_geoscience::append_geoscience;
use super::celestial_body_radiation::append_radiation_and_magnetism;
use super::celestial_body_volcanism::append_volcanism;
use super::celestial_body_weather::append_reference_weather;
use super::orbit_preview::{OrbitPreview, preview_from_pool_cancelled};
use crate::{RocketError, RocketResult};
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::units::Duration;
use astronomicon_db::SqlitePool;
use rocketcon_db::repositories::vehicle_physical_state;
use std::path::Path;
use uuid::Uuid;

pub struct ForecastReader {
    pool: SqlitePool,
    vehicle_id: Uuid,
    universe_epoch: Duration,
    catalog: EphemerisCatalog,
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
        let state = vehicle_physical_state::get_by_vehicle_id(&pool, &vehicle_id)
            .await?
            .ok_or_else(|| RocketError::Generic("atlas vehicle missing".into()))?;
        let environment = rocketcon_app::environment::load_environment_snapshot(
            &pool,
            state.reference_body_id(),
            universe_epoch,
            state.captured_at_epoch(),
        )
        .await?;
        let catalog = EphemerisCatalog::load(&pool, environment.system_id).await?;
        Ok(Self {
            pool,
            vehicle_id,
            universe_epoch,
            catalog,
        })
    }

    pub async fn celestial_atlas(&self) -> RocketResult<CelestialAtlas> {
        let state = vehicle_physical_state::get_by_vehicle_id(&self.pool, &self.vehicle_id)
            .await?
            .ok_or_else(|| RocketError::Generic("atlas vehicle missing".into()))?;
        catalog_at(
            &self.catalog,
            state.captured_total_epoch(),
            state.reference_body_id(),
        )
    }

    pub async fn body_details(&self, id: Uuid) -> RocketResult<CelestialBodyDetails> {
        let state = vehicle_physical_state::get_by_vehicle_id(&self.pool, &self.vehicle_id)
            .await?
            .ok_or_else(|| RocketError::Generic("atlas vehicle missing".into()))?;
        let mut details = selected_body_details(&self.pool, &self.catalog, id).await?;
        let radiative = append_reference_environment(
            &mut details,
            &self.catalog,
            id,
            self.universe_epoch,
            state.captured_at_epoch(),
        );
        if let Some(context) = radiative.as_ref() {
            append_radiation_and_magnetism(
                &mut details,
                &self.pool,
                &self.catalog,
                id,
                self.universe_epoch,
                state.captured_at_epoch(),
                context,
            )
            .await;
        }
        if let Some(body) = self.catalog.planets().iter().find(|body| body.id() == id) {
            append_atmospheric_physics(&mut details, &self.pool, body).await;
            if let Some(context) = radiative.as_ref() {
                if let Some(winds) = append_reference_winds(
                    &mut details, &self.pool, &self.catalog, body, context
                ).await {
                    append_reference_weather(&mut details, &self.pool, body, &winds).await;
                }
            }
            append_geoscience(
                &mut details,
                &self.pool,
                id,
                body.kind(),
                self.universe_epoch,
                state.captured_at_epoch(),
            )
            .await;
            append_volcanism(
                &mut details,
                &self.pool,
                id,
                body.kind(),
                self.universe_epoch,
                state.captured_at_epoch(),
            )
            .await;
        }
        Ok(details)
    }

    pub async fn preview(&self, under_thrust: bool) -> RocketResult<OrbitPreview> {
        self.preview_cancelled(under_thrust, &mut || false, &mut |_| {})
            .await
    }

    pub async fn preview_cancelled(
        &self,
        under_thrust: bool,
        cancelled: &mut impl FnMut() -> bool,
        progress: &mut impl FnMut(OrbitPreview),
    ) -> RocketResult<OrbitPreview> {
        preview_from_pool_cancelled(
            &self.pool,
            self.vehicle_id,
            self.universe_epoch,
            None,
            under_thrust,
            cancelled,
            progress,
        )
        .await
    }

    pub async fn close(self) {
        self.pool.close().await;
    }
}
