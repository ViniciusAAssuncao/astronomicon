use crate::error::{RocketDomainError, RocketDomainResult};
use astronomicon_core::domain::{Atmosphere, Planet, Star};
use astronomicon_core::units::{Duration, Length, Position};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct EnvironmentSnapshot {
    pub star: Star,
    pub planet: Planet,
    pub atmosphere: Option<Atmosphere>,
    pub star_position: Position,
    pub planet_position: Position,
    pub system_id: Uuid,
    pub captured_universe_epoch: Duration,
    pub captured_at_epoch: Duration,
}

impl EnvironmentSnapshot {
    pub fn new(
        star: Star,
        planet: Planet,
        atmosphere: Option<Atmosphere>,
        star_position: Position,
        planet_position: Position,
        system_id: Uuid,
        captured_universe_epoch: Duration,
        captured_at_epoch: Duration,
    ) -> RocketDomainResult<Self> {
        if !captured_universe_epoch.value().is_finite() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "captured_universe_epoch".to_string(),
                reason: "value must be finite".to_string(),
            });
        }

        if !captured_at_epoch.value().is_finite() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "captured_at_epoch".to_string(),
                reason: "value must be finite".to_string(),
            });
        }

        if let Some(ref atm) = atmosphere {
            if atm.planet_id() != planet.id() {
                return Err(RocketDomainError::InvalidInvariant {
                    field: "atmosphere".to_string(),
                    reason: format!(
                        "atmosphere planet_id '{}' does not match planet id '{}'",
                        atm.planet_id(),
                        planet.id()
                    ),
                });
            }
        }

        Ok(Self {
            star,
            planet,
            atmosphere,
            star_position,
            planet_position,
            system_id,
            captured_universe_epoch,
            captured_at_epoch,
        })
    }

    pub fn captured_total_epoch(&self) -> Duration {
        self.captured_universe_epoch + self.captured_at_epoch
    }

    pub fn is_stale(
        &self,
        current_total_epoch: Duration,
        current_planet_position: Position,
        max_epoch_delta: Duration,
        max_position_delta: Length,
    ) -> bool {
        let epoch_diff = (current_total_epoch.value() - self.captured_total_epoch().value()).abs();
        if epoch_diff > max_epoch_delta.value() {
            return true;
        }

        let pos_diff = (current_planet_position - self.planet_position).magnitude();
        if pos_diff.value() > max_position_delta.value() {
            return true;
        }

        false
    }
}
