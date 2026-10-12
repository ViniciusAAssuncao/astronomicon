use super::{resolve_advective_surface_temperature_inner, resolve_top_of_atmosphere_irradiance};
use crate::climate::circulation::PlanetaryCirculationDiagnostic;
use crate::error::AppResult;
use crate::hierarchy::find_parent_star;
use astronomicon_core::domain::Planet;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::climate::{
    day_length_half_angle, local_radiative_equilibrium_temperature, mean_daily_insolation_factor,
    solar_declination,
};
use astronomicon_core::math::gravity::combined_gravitational_parameter;
use astronomicon_core::math::kepler::true_anomaly_at_epoch;
use astronomicon_core::units::{Angle, Duration, Irradiance, Temperature};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{atmosphere_repository, planet_repository};
use uuid::Uuid;

mod body;
mod epoch;

pub use body::ClimateBodyInputs;
use epoch::EpochClimateData;

pub(super) struct SurfaceTemperatureInputs {
    declination: Angle,
    top_irradiance: Irradiance,
    greenhouse: Temperature,
    bond_albedo: f64,
}

impl SurfaceTemperatureInputs {
    pub(super) async fn load(
        pool: &SqlitePool,
        planet_id: Uuid,
        universe_epoch: Duration,
        at_epoch: Duration,
    ) -> AppResult<Self> {
        let planet_row = planet_repository::get_by_id(pool, &planet_id)
            .await?
            .ok_or_else(|| DomainError::InvalidInvariant {
                field: "planet_id".to_string(),
                reason: format!("planet '{}' not found", planet_id),
            })?;
        let planet = Planet::try_from(planet_row)?;
        let star = find_parent_star(pool, planet.orbital_parent()).await?;
        let obliquity = planet.obliquity().unwrap_or_else(|| Angle::new(0.0));
        let solstice_true_anomaly = planet
            .solstice_true_anomaly()
            .unwrap_or_else(|| Angle::new(0.0));
        let orbital_elements =
            planet
                .orbital_elements()
                .ok_or_else(|| DomainError::InvalidInvariant {
                    field: "orbital_elements".to_string(),
                    reason: "planet does not have orbital elements".to_string(),
                })?;
        let bond_albedo = planet.bond_albedo().unwrap_or(0.3);
        let total_epoch = universe_epoch + at_epoch;
        let mu = combined_gravitational_parameter(planet.mass(), star.mass());
        let true_anomaly = true_anomaly_at_epoch(&orbital_elements, mu, total_epoch)?;
        let declination = solar_declination(
            obliquity,
            orbital_elements.argument_of_periapsis(),
            solstice_true_anomaly,
            true_anomaly,
        );
        let top_irradiance =
            resolve_top_of_atmosphere_irradiance(pool, &planet, &star, universe_epoch, at_epoch)
                .await?;
        let greenhouse = match atmosphere_repository::get_by_planet_id(pool, &planet_id).await? {
            Some(atmosphere) => atmosphere.greenhouse_effect(),
            None => Temperature::new(0.0),
        };
        Ok(Self {
            declination,
            top_irradiance,
            greenhouse,
            bond_albedo,
        })
    }

    pub(super) fn local_surface_temperature(&self, latitude: Angle) -> Temperature {
        let half_angle = day_length_half_angle(latitude, self.declination);
        let insolation_factor =
            mean_daily_insolation_factor(latitude, self.declination, half_angle);
        let local_insolation = self.top_irradiance * insolation_factor;
        local_radiative_equilibrium_temperature(local_insolation, self.bond_albedo)
            + self.greenhouse
    }
}

pub struct AdvectiveTemperatureContext {
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
    global_mean: Temperature,
    circulation: PlanetaryCirculationDiagnostic,
    surface: SurfaceTemperatureInputs,
}

impl AdvectiveTemperatureContext {
    pub async fn load(
        pool: &SqlitePool,
        planet_id: Uuid,
        universe_epoch: Duration,
        at_epoch: Duration,
    ) -> AppResult<Self> {
        let body = ClimateBodyInputs::load(pool, planet_id).await?;
        Self::load_with_body(pool, &body, universe_epoch, at_epoch).await
    }

    pub async fn load_in_existing_transaction(
        pool: &SqlitePool,
        planet_id: Uuid,
        universe_epoch: Duration,
        at_epoch: Duration,
    ) -> AppResult<Self> {
        let body = ClimateBodyInputs::load_in_existing_transaction(pool, planet_id).await?;
        Self::load_with_body(pool, &body, universe_epoch, at_epoch).await
    }

    pub async fn load_with_body(
        pool: &SqlitePool,
        body: &ClimateBodyInputs,
        universe_epoch: Duration,
        at_epoch: Duration,
    ) -> AppResult<Self> {
        let EpochClimateData {
            global_mean,
            circulation,
            surface,
        } = EpochClimateData::resolve(pool, body, universe_epoch, at_epoch).await?;
        Ok(Self {
            planet_id: body.planet_id(),
            universe_epoch,
            at_epoch,
            global_mean,
            circulation,
            surface,
        })
    }

    pub async fn temperature_at_latitude(
        &self,
        pool: &SqlitePool,
        latitude: Angle,
    ) -> AppResult<Temperature> {
        resolve_advective_surface_temperature_inner(
            pool,
            self.planet_id,
            latitude,
            self.universe_epoch,
            self.at_epoch,
            Some(&self.circulation),
            Some(self.global_mean),
            Some(&self.surface),
        )
        .await
    }

    pub fn planet_id(&self) -> Uuid {
        self.planet_id
    }

    pub fn universe_epoch(&self) -> Duration {
        self.universe_epoch
    }

    pub fn at_epoch(&self) -> Duration {
        self.at_epoch
    }
}
