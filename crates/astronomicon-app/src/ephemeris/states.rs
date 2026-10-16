use crate::ephemeris::positions::compute_system_positions;
use crate::ephemeris::prepared_positions::PreparedPositionLookup;
use crate::error::AppResult;
use astronomicon_core::domain::{
    Barycenter, MinorPlanet, OrbitalElements, OrbitalParent, Planet, Star,
};
use astronomicon_core::error::{DomainError, DomainResult};
use astronomicon_core::math::gravity::{
    calculate_effective_mass, calculate_parent_effective_mass, combined_gravitational_parameter,
};
use astronomicon_core::units::{Duration, Mass, Position, VelocityVector};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{
    barycenter_repository, minor_planet_repository, planet_repository, star_repository,
};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Copy, Debug)]
pub struct CelestialState {
    pub position: Position,
    pub velocity: VelocityVector,
    pub epoch: Duration,
}

pub struct EphemerisCatalog {
    pub system_id: Uuid,
    stars: Vec<Star>,
    planets: Vec<Planet>,
    barycenters: Vec<Barycenter>,
    minor_planets: Vec<MinorPlanet>,
}

impl EphemerisCatalog {
    pub async fn load(pool: &SqlitePool, system_id: Uuid) -> AppResult<Self> {
        let stars = star_repository::list_by_system(pool, &system_id)
            .await?
            .into_iter()
            .map(Star::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        let planets = planet_repository::list_by_system(pool, &system_id)
            .await?
            .into_iter()
            .map(Planet::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        let barycenters = barycenter_repository::list_by_system(pool, &system_id)
            .await?
            .into_iter()
            .map(Barycenter::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        let minor_planets = minor_planet_repository::list_by_system(pool, &system_id)
            .await?
            .into_iter()
            .map(MinorPlanet::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            system_id,
            stars,
            planets,
            barycenters,
            minor_planets,
        })
    }

    pub fn stars(&self) -> &[Star] {
        &self.stars
    }

    pub fn planets(&self) -> &[Planet] {
        &self.planets
    }

    pub fn radius_of_body(&self, id: Uuid) -> Option<f64> {
        self.stars
            .iter()
            .find(|body| body.id() == id)
            .and_then(|body| body.radius())
            .map(|radius| radius.value())
            .or_else(|| {
                self.planets
                    .iter()
                    .find(|body| body.id() == id)
                    .and_then(|body| body.equatorial_radius().or(body.polar_radius()))
                    .map(|radius| radius.value())
            })
            .filter(|radius| radius.is_finite() && *radius > 0.0)
    }

    pub fn gravitating_bodies(&self) -> Vec<(Uuid, Mass)> {
        self.stars
            .iter()
            .map(|body| (body.id(), body.mass()))
            .chain(self.planets.iter().map(|body| (body.id(), body.mass())))
            .chain(
                self.minor_planets
                    .iter()
                    .map(|body| (body.id(), body.mass())),
            )
            .collect()
    }

    pub fn mass_of_body(&self, id: Uuid) -> Option<Mass> {
        self.stars
            .iter()
            .find(|body| body.id() == id)
            .map(|body| body.mass())
            .or_else(|| {
                self.planets
                    .iter()
                    .find(|body| body.id() == id)
                    .map(|body| body.mass())
            })
    }

    pub fn prepared_positions(&self) -> DomainResult<PreparedPositionLookup<'_>> {
        PreparedPositionLookup::new(
            &self.stars,
            &self.planets,
            &self.barycenters,
            &self.minor_planets,
        )
    }

    pub fn positions(&self, epoch: Duration) -> DomainResult<HashMap<Uuid, Position>> {
        compute_system_positions(
            &self.stars,
            &self.planets,
            &self.barycenters,
            &self.minor_planets,
            epoch,
        )
    }

    pub fn positions_at_offset(
        &self,
        epoch: Duration,
        offset_seconds: f64,
    ) -> DomainResult<HashMap<Uuid, Position>> {
        if !epoch.value().is_finite()
            || !offset_seconds.is_finite()
            || !(epoch.value() + offset_seconds).is_finite()
        {
            return Err(DomainError::InvalidInvariant {
                field: "ephemeris_time".into(),
                reason: "invalid base epoch or relative offset".into(),
            });
        }
        if offset_seconds == 0.0 {
            return self.positions(epoch);
        }
        crate::ephemeris::positions::compute_system_positions_at_offset(
            &self.stars,
            &self.planets,
            &self.barycenters,
            &self.minor_planets,
            epoch,
            offset_seconds,
        )
    }
    pub fn positions_for_ids_at_offset(
        &self,
        epoch: Duration,
        offset_seconds: f64,
        ids: &[Uuid],
    ) -> DomainResult<HashMap<Uuid, Position>> {
        if !epoch.value().is_finite()
            || !offset_seconds.is_finite()
            || !(epoch.value() + offset_seconds).is_finite()
        {
            return Err(DomainError::InvalidInvariant {
                field: "ephemeris_time".into(),
                reason: "invalid base epoch or relative offset".into(),
            });
        }
        crate::ephemeris::positions::compute_selected_system_positions_at_offset(
            &self.stars,
            &self.planets,
            &self.barycenters,
            &self.minor_planets,
            epoch,
            offset_seconds,
            ids,
        )
    }

    pub fn states(&self, epoch: Duration) -> AppResult<HashMap<Uuid, CelestialState>> {
        let center = self.positions(epoch)?;
        let Some(step) = self.differentiation_step(epoch)? else {
            return Ok(center
                .into_iter()
                .map(|(id, position)| {
                    (
                        id,
                        CelestialState {
                            position,
                            velocity: VelocityVector::zero(),
                            epoch,
                        },
                    )
                })
                .collect());
        };
        let before2 = self.positions(epoch - Duration::new(2.0 * step))?;
        let before1 = self.positions(epoch - Duration::new(step))?;
        let after1 = self.positions(epoch + Duration::new(step))?;
        let after2 = self.positions(epoch + Duration::new(2.0 * step))?;
        center
            .into_iter()
            .map(|(id, position)| {
                let a = before2.get(&id).ok_or_else(|| missing_body(id))?;
                let b = before1.get(&id).ok_or_else(|| missing_body(id))?;
                let c = after1.get(&id).ok_or_else(|| missing_body(id))?;
                let d = after2.get(&id).ok_or_else(|| missing_body(id))?;
                let raw = (a.raw() - b.raw() * 8.0 + c.raw() * 8.0 - d.raw()) / (12.0 * step);
                if ![raw.0, raw.1, raw.2].iter().all(|value| value.is_finite()) {
                    return Err(DomainError::InvalidInvariant {
                        field: "ephemeris_velocity".into(),
                        reason: format!("non-finite derivative for body '{id}'"),
                    }
                    .into());
                }
                Ok((
                    id,
                    CelestialState {
                        position,
                        velocity: VelocityVector::from_raw(raw),
                        epoch,
                    },
                ))
            })
            .collect()
    }

    fn differentiation_step(&self, epoch: Duration) -> DomainResult<Option<f64>> {
        let stars: HashMap<Uuid, &Star> = self.stars.iter().map(|item| (item.id(), item)).collect();
        let planets: HashMap<Uuid, &Planet> =
            self.planets.iter().map(|item| (item.id(), item)).collect();
        let barycenters: HashMap<Uuid, &Barycenter> = self
            .barycenters
            .iter()
            .map(|item| (item.id(), item))
            .collect();
        let minors: HashMap<Uuid, &MinorPlanet> = self
            .minor_planets
            .iter()
            .map(|item| (item.id(), item))
            .collect();
        let mut shortest = f64::INFINITY;
        let update = |parent: OrbitalParent,
                      elements: Option<OrbitalElements>,
                      mass: Mass,
                      shortest: &mut f64| {
            if matches!(parent, OrbitalParent::Fixed) {
                return Ok(());
            }
            let elements = elements.ok_or_else(|| DomainError::InvalidInvariant {
                field: "orbital_elements".into(),
                reason: "orbiting body has no orbital elements".into(),
            })?;
            let parent_mass =
                calculate_parent_effective_mass(&parent, &stars, &planets, &barycenters, &minors)?;
            let mu = combined_gravitational_parameter(mass, parent_mass).value();
            let periapsis =
                (elements.semi_major_axis().value() * (1.0 - elements.eccentricity())).abs();
            if mu <= 0.0 || !mu.is_finite() || !periapsis.is_finite() || periapsis <= 0.0 {
                return Err(DomainError::InvalidInvariant {
                    field: "orbital_timescale".into(),
                    reason: "invalid mass or periapsis for ephemeris derivative".into(),
                });
            }
            *shortest = shortest.min((periapsis.powi(3) / mu).sqrt());
            Ok(())
        };
        for item in &self.stars {
            update(
                item.orbital_parent(),
                item.orbital_elements(),
                item.mass(),
                &mut shortest,
            )?;
        }
        for item in &self.planets {
            update(
                item.orbital_parent(),
                item.orbital_elements(),
                item.mass(),
                &mut shortest,
            )?;
        }
        for item in &self.minor_planets {
            update(
                item.orbital_parent(),
                item.orbital_elements(),
                item.mass(),
                &mut shortest,
            )?;
        }
        for item in &self.barycenters {
            let primary_mass =
                calculate_effective_mass(&item.member_primary(), &stars, &planets, &barycenters)?;
            let secondary_mass =
                calculate_effective_mass(&item.member_secondary(), &stars, &planets, &barycenters)?;
            let mass = Mass::new(primary_mass.value() + secondary_mass.value());
            update(
                item.orbital_parent(),
                item.external_orbital_elements(),
                mass,
                &mut shortest,
            )?;
            let elements = item.internal_orbital_elements();
            let mu = combined_gravitational_parameter(primary_mass, secondary_mass).value();
            let periapsis = elements.semi_major_axis().value() * (1.0 - elements.eccentricity());
            if mu <= 0.0 || !mu.is_finite() || periapsis <= 0.0 || !periapsis.is_finite() {
                return Err(DomainError::InvalidInvariant {
                    field: "barycenter_timescale".into(),
                    reason: "invalid internal orbit for ephemeris derivative".into(),
                });
            }
            shortest = shortest.min((periapsis.powi(3) / mu).sqrt());
        }
        if !shortest.is_finite() {
            return Ok(None);
        }
        let truncation_step = shortest * f64::EPSILON.powf(1.0 / 5.0);
        let representable_step = epoch.value().abs().max(shortest) * f64::EPSILON.sqrt();
        let step = truncation_step.max(representable_step);
        if step.is_finite()
            && step > 0.0
            && epoch.value().is_finite()
            && (epoch.value() + 2.0 * step).is_finite()
        {
            Ok(Some(step))
        } else {
            Err(DomainError::InvalidInvariant {
                field: "ephemeris_epoch".into(),
                reason: "epoch outside differentiable range".into(),
            })
        }
    }
}

fn missing_body(id: Uuid) -> DomainError {
    DomainError::InvalidInvariant {
        field: "ephemeris_body".into(),
        reason: format!("body '{id}' missing from differential sample"),
    }
}
