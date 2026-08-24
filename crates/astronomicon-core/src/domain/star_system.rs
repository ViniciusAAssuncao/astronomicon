use crate::domain::validation::{validate_finite, validate_not_empty, validate_positive_finite};
use crate::error::DomainResult;
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
        validate_not_empty(&name, "name")?;

        if let Some(dist) = distance_from_sun {
            validate_positive_finite(dist.value(), "distance_from_sun")?;
        }

        if let Some(ra) = right_ascension {
            validate_finite(ra.value(), "right_ascension")?;
        }

        if let Some(dec) = declination {
            validate_finite(dec.value(), "declination")?;
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
