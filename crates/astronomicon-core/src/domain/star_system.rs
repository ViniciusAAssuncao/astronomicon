use crate::error::{DomainError, DomainResult};
use crate::units::{Angle, Length};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StarSystem {
    id: Uuid,
    name: String,
    right_ascension: Option<Angle>,
    declination: Option<Angle>,
    distance_from_sun: Option<Length>,
}

impl StarSystem {
    pub fn new(
        id: Uuid,
        name: String,
        right_ascension: Option<Angle>,
        declination: Option<Angle>,
        distance_from_sun: Option<Length>,
    ) -> DomainResult<Self> {
        if name.trim().is_empty() {
            return Err(DomainError::InvalidInvariant {
                field: "name".to_string(),
                reason: "cannot be empty".to_string(),
            });
        }

        if let Some(dist) = distance_from_sun {
            if !dist.value().is_finite() || dist.value() <= 0.0 {
                return Err(DomainError::InvalidInvariant {
                    field: "distance_from_sun".to_string(),
                    reason: "must be positive and finite".to_string(),
                });
            }
        }

        if let Some(ra) = right_ascension {
            if !ra.value().is_finite() {
                return Err(DomainError::InvalidInvariant {
                    field: "right_ascension".to_string(),
                    reason: "must be finite".to_string(),
                });
            }
        }

        if let Some(dec) = declination {
            if !dec.value().is_finite() {
                return Err(DomainError::InvalidInvariant {
                    field: "declination".to_string(),
                    reason: "must be finite".to_string(),
                });
            }
        }

        Ok(Self {
            id,
            name,
            right_ascension,
            declination,
            distance_from_sun,
        })
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn right_ascension(&self) -> Option<Angle> {
        self.right_ascension
    }

    pub fn declination(&self) -> Option<Angle> {
        self.declination
    }

    pub fn distance_from_sun(&self) -> Option<Length> {
        self.distance_from_sun
    }
}
