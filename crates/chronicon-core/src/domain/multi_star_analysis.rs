use crate::domain::multi_star_classification::MultiStarOrbitType;
use crate::math::multi_star::{
    binary_orbital_period, find_barycenter_containing_member, second_sun_synodic_period,
};
use astronomicon_core::domain::{Barycenter, OrbitalParent, Planet, Star};
use astronomicon_core::error::DomainResult;
use astronomicon_core::units::{Duration, Length};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiStarSystemInfo {
    pub orbit_type: MultiStarOrbitType,
    pub barycenter_id: Option<Uuid>,
    pub host_star_id: Option<Uuid>,
    pub companion_member_id: Option<Uuid>,
    pub binary_orbital_period: Option<Duration>,
    pub second_sun_synodic_period: Option<Duration>,
    pub binary_semi_major_axis: Option<Length>,
    pub binary_eccentricity: Option<f64>,
}

impl MultiStarSystemInfo {
    pub fn new(
        orbit_type: MultiStarOrbitType,
        barycenter_id: Option<Uuid>,
        host_star_id: Option<Uuid>,
        companion_member_id: Option<Uuid>,
        binary_orbital_period: Option<Duration>,
        second_sun_synodic_period: Option<Duration>,
        binary_semi_major_axis: Option<Length>,
        binary_eccentricity: Option<f64>,
    ) -> Self {
        Self {
            orbit_type,
            barycenter_id,
            host_star_id,
            companion_member_id,
            binary_orbital_period,
            second_sun_synodic_period,
            binary_semi_major_axis,
            binary_eccentricity,
        }
    }

    pub fn single_star() -> Self {
        Self::new(
            MultiStarOrbitType::SingleStar,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    pub fn non_stellar() -> Self {
        Self::new(
            MultiStarOrbitType::NonStellarParent,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    pub fn orbit_type(&self) -> MultiStarOrbitType {
        self.orbit_type
    }

    pub fn barycenter_id(&self) -> Option<Uuid> {
        self.barycenter_id
    }

    pub fn host_star_id(&self) -> Option<Uuid> {
        self.host_star_id
    }

    pub fn companion_member_id(&self) -> Option<Uuid> {
        self.companion_member_id
    }

    pub fn binary_orbital_period(&self) -> Option<Duration> {
        self.binary_orbital_period
    }

    pub fn second_sun_synodic_period(&self) -> Option<Duration> {
        self.second_sun_synodic_period
    }

    pub fn binary_semi_major_axis(&self) -> Option<Length> {
        self.binary_semi_major_axis
    }

    pub fn binary_eccentricity(&self) -> Option<f64> {
        self.binary_eccentricity
    }
}

pub fn analyze_multi_star_system(
    planet: &Planet,
    planet_orbital_period: Option<Duration>,
    stars: &HashMap<Uuid, &Star>,
    planets: &HashMap<Uuid, &Planet>,
    barycenters: &HashMap<Uuid, &Barycenter>,
) -> DomainResult<MultiStarSystemInfo> {
    match planet.orbital_parent() {
        OrbitalParent::Barycenter(bary_id) => {
            if let Some(bary) = barycenters.get(&bary_id) {
                let bin_period = binary_orbital_period(bary, stars, planets, barycenters)?;
                let synodic = match (planet_orbital_period, bin_period) {
                    (Some(p_orb), Some(b_orb)) => second_sun_synodic_period(p_orb, b_orb),
                    _ => None,
                };
                let internal_elements = bary.internal_orbital_elements();

                Ok(MultiStarSystemInfo::new(
                    MultiStarOrbitType::PType,
                    Some(bary_id),
                    None,
                    None,
                    bin_period,
                    synodic,
                    Some(internal_elements.semi_major_axis()),
                    Some(internal_elements.eccentricity()),
                ))
            } else {
                Ok(MultiStarSystemInfo::new(
                    MultiStarOrbitType::PType,
                    Some(bary_id),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ))
            }
        }
        OrbitalParent::Star(star_id) => {
            if let Some(bary) = find_barycenter_containing_member(star_id, barycenters) {
                let bin_period = binary_orbital_period(bary, stars, planets, barycenters)?;
                let synodic = match (planet_orbital_period, bin_period) {
                    (Some(p_orb), Some(b_orb)) => second_sun_synodic_period(p_orb, b_orb),
                    _ => None,
                };
                let internal_elements = bary.internal_orbital_elements();

                let companion_id = if bary.member_primary().id() == star_id {
                    bary.member_secondary().id()
                } else {
                    bary.member_primary().id()
                };

                Ok(MultiStarSystemInfo::new(
                    MultiStarOrbitType::SType,
                    Some(bary.id()),
                    Some(star_id),
                    Some(companion_id),
                    bin_period,
                    synodic,
                    Some(internal_elements.semi_major_axis()),
                    Some(internal_elements.eccentricity()),
                ))
            } else {
                Ok(MultiStarSystemInfo::new(
                    MultiStarOrbitType::SingleStar,
                    None,
                    Some(star_id),
                    None,
                    None,
                    None,
                    None,
                    None,
                ))
            }
        }
        OrbitalParent::Planet(_) | OrbitalParent::MinorPlanet(_) => {
            Ok(MultiStarSystemInfo::non_stellar())
        }
        OrbitalParent::Fixed => Ok(MultiStarSystemInfo::single_star()),
    }
}