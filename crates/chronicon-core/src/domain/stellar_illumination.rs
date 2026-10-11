use crate::domain::rotation_classification::RotationClassification;
use crate::math::multi_star::apparent_motion::star_individual_solar_day;
use crate::math::multi_star::grand_cycle::find_polysolar_grand_cycle;
use crate::math::multi_star::hierarchy_traversal::collect_gravitationally_linked_stars;
use crate::math::multi_star::relevance::evaluate_star_relevance;
use astronomicon_core::domain::{Barycenter, MinorPlanet, Planet, Star};
use astronomicon_core::error::DomainResult;
use astronomicon_core::units::{Duration, Irradiance, Length};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolysolarClassification {
    SingleStar,
    Binary,
    Trinary,
    HierarchicalMultiple,
}

impl PolysolarClassification {
    pub fn is_single_star(&self) -> bool {
        matches!(self, Self::SingleStar)
    }

    pub fn is_binary(&self) -> bool {
        matches!(self, Self::Binary)
    }

    pub fn is_trinary(&self) -> bool {
        matches!(self, Self::Trinary)
    }

    pub fn is_multiple(&self) -> bool {
        !matches!(self, Self::SingleStar)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StellarIlluminationComponent {
    pub star_id: Uuid,
    pub star_name: String,
    pub apparent_magnitude: f64,
    pub irradiance: Irradiance,
    pub distance: Length,
    pub is_primary: bool,
    pub is_relevant: bool,
    pub individual_solar_day: Option<Duration>,
    pub rotation_classification: RotationClassification,
}

impl StellarIlluminationComponent {
    pub fn new(
        star_id: Uuid,
        star_name: impl Into<String>,
        apparent_magnitude: f64,
        irradiance: Irradiance,
        distance: Length,
        is_primary: bool,
        is_relevant: bool,
        individual_solar_day: Option<Duration>,
        rotation_classification: RotationClassification,
    ) -> Self {
        Self {
            star_id,
            star_name: star_name.into(),
            apparent_magnitude,
            irradiance,
            distance,
            is_primary,
            is_relevant,
            individual_solar_day,
            rotation_classification,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolysolarDayAnalysis {
    pub planet_id: Uuid,
    pub components: Vec<StellarIlluminationComponent>,
    pub classification: PolysolarClassification,
    pub grand_cycle: Option<Duration>,
    pub primary_star_id: Option<Uuid>,
}

impl PolysolarDayAnalysis {
    pub fn new(
        planet_id: Uuid,
        components: Vec<StellarIlluminationComponent>,
        classification: PolysolarClassification,
        grand_cycle: Option<Duration>,
        primary_star_id: Option<Uuid>,
    ) -> Self {
        Self {
            planet_id,
            components,
            classification,
            grand_cycle,
            primary_star_id,
        }
    }

    pub fn planet_id(&self) -> Uuid {
        self.planet_id
    }

    pub fn components(&self) -> &[StellarIlluminationComponent] {
        &self.components
    }

    pub fn classification(&self) -> PolysolarClassification {
        self.classification
    }

    pub fn grand_cycle(&self) -> Option<Duration> {
        self.grand_cycle
    }

    pub fn primary_star_id(&self) -> Option<Uuid> {
        self.primary_star_id
    }

    pub fn relevant_components(&self) -> impl Iterator<Item = &StellarIlluminationComponent> {
        self.components.iter().filter(|c| c.is_relevant)
    }
}

pub fn analyze_polysolar_day(
    planet: &Planet,
    stars: &HashMap<Uuid, &Star>,
    planets: &HashMap<Uuid, &Planet>,
    barycenters: &HashMap<Uuid, &Barycenter>,
    minor_planets: &HashMap<Uuid, &MinorPlanet>,
    max_denominator: Option<u32>,
) -> DomainResult<PolysolarDayAnalysis> {
    let linked_stars =
        collect_gravitationally_linked_stars(planet, stars, planets, barycenters, minor_planets);

    let mut components = Vec::with_capacity(linked_stars.len());

    for star in linked_stars {
        let relevance =
            evaluate_star_relevance(planet, star, stars, planets, barycenters, minor_planets)?;
        let (individual_day, rot_class) = star_individual_solar_day(
            planet,
            star.id(),
            stars,
            planets,
            barycenters,
            minor_planets,
        );

        components.push(StellarIlluminationComponent::new(
            star.id(),
            star.name(),
            relevance.apparent_magnitude,
            relevance.irradiance,
            relevance.distance,
            false,
            relevance.is_relevant,
            individual_day,
            rot_class,
        ));
    }

    components.sort_by(|a, b| {
        a.apparent_magnitude
            .partial_cmp(&b.apparent_magnitude)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if let Some(first) = components.first_mut() {
        first.is_primary = true;
    }

    let primary_star_id = components.iter().find(|c| c.is_primary).map(|c| c.star_id);

    let relevant_count = components.iter().filter(|c| c.is_relevant).count();

    let classification = match relevant_count {
        0 | 1 => PolysolarClassification::SingleStar,
        2 => PolysolarClassification::Binary,
        3 => PolysolarClassification::Trinary,
        _ => PolysolarClassification::HierarchicalMultiple,
    };

    let grand_cycle = if relevant_count > 1 {
        let primary_day = components
            .iter()
            .find(|c| c.is_primary)
            .and_then(|c| c.individual_solar_day);

        let companion_days: Vec<Duration> = components
            .iter()
            .filter(|c| c.is_relevant && !c.is_primary)
            .filter_map(|c| c.individual_solar_day)
            .collect();

        match primary_day {
            Some(pri_day) if !companion_days.is_empty() => {
                find_polysolar_grand_cycle(pri_day, &companion_days, max_denominator)
            }
            _ => None,
        }
    } else {
        None
    };

    Ok(PolysolarDayAnalysis::new(
        planet.id(),
        components,
        classification,
        grand_cycle,
        primary_star_id,
    ))
}