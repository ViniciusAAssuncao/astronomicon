use crate::error::AppResult;
use crate::hierarchy::find_parent_star_on_connection;
use astronomicon_core::domain::{Atmosphere, Hydrosphere, Planet, Star};
use astronomicon_core::error::DomainError;
use astronomicon_db::error::DbError;
use astronomicon_db::repositories::{
    atmosphere_repository, hydrosphere_repository, planet_repository,
};
use astronomicon_db::{SqliteConnection, SqlitePool};
use uuid::Uuid;

#[derive(Debug, PartialEq)]
pub struct ClimateBodyInputs {
    pub(super) planet: Planet,
    pub(super) star: Star,
    pub(super) atmosphere: Option<Atmosphere>,
    pub(super) hydrosphere: Option<Hydrosphere>,
}

impl ClimateBodyInputs {
    pub async fn load(pool: &SqlitePool, planet_id: Uuid) -> AppResult<Self> {
        let mut transaction = pool.begin().await.map_err(DbError::from)?;
        let body = Self::load_from_connection(&mut transaction, planet_id).await?;
        transaction.rollback().await.map_err(DbError::from)?;
        Ok(body)
    }

    pub async fn load_in_existing_transaction(
        pool: &SqlitePool,
        planet_id: Uuid,
    ) -> AppResult<Self> {
        let mut connection = pool.acquire().await.map_err(DbError::from)?;
        Self::load_from_connection(&mut connection, planet_id).await
    }

    async fn load_from_connection(
        connection: &mut SqliteConnection,
        planet_id: Uuid,
    ) -> AppResult<Self> {
        let planet_row = planet_repository::get_by_id_on_connection(connection, &planet_id)
            .await?
            .ok_or_else(|| DomainError::InvalidInvariant {
                field: "planet_id".to_string(),
                reason: format!("planet '{}' not found", planet_id),
            })?;
        let planet = Planet::try_from(planet_row)?;
        let star = find_parent_star_on_connection(connection, planet.orbital_parent()).await?;
        let atmosphere =
            atmosphere_repository::get_by_planet_id_on_connection(connection, &planet_id).await?;
        let hydrosphere =
            hydrosphere_repository::get_by_planet_id_on_connection(connection, &planet_id).await?;
        Ok(Self {
            planet,
            star,
            atmosphere,
            hydrosphere,
        })
    }

    pub fn planet_id(&self) -> Uuid {
        self.planet.id()
    }

    pub async fn refresh_if_changed(&mut self, pool: &SqlitePool) -> AppResult<bool> {
        let current = Self::load(pool, self.planet_id()).await?;
        if *self == current {
            return Ok(false);
        }
        *self = current;
        Ok(true)
    }

}
