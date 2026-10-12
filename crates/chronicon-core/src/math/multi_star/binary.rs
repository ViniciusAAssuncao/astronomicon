use crate::math::synodic::synodic_beat_period;
use astronomicon_core::domain::{Barycenter, Planet, Star};
use astronomicon_core::error::DomainResult;
use astronomicon_core::math::gravity::{
    calculate_effective_mass, combined_gravitational_parameter,
};
use astronomicon_core::math::kepler::orbital_period;
use astronomicon_core::units::{Duration, Mass};
use std::collections::HashMap;
use uuid::Uuid;

pub fn find_barycenter_containing_member<'a>(
    member_id: Uuid,
    barycenters: &'a HashMap<Uuid, &'a Barycenter>,
) -> Option<&'a Barycenter> {
    for bary in barycenters.values() {
        if bary.member_primary().id() == member_id || bary.member_secondary().id() == member_id {
            return Some(*bary);
        }
    }
    None
}

pub fn binary_member_masses(
    barycenter: &Barycenter,
    stars: &HashMap<Uuid, &Star>,
    planets: &HashMap<Uuid, &Planet>,
    barycenters: &HashMap<Uuid, &Barycenter>,
) -> DomainResult<(Mass, Mass)> {
    let m1 = calculate_effective_mass(&barycenter.member_primary(), stars, planets, barycenters)?;
    let m2 = calculate_effective_mass(&barycenter.member_secondary(), stars, planets, barycenters)?;
    Ok((m1, m2))
}

pub fn binary_orbital_period(
    barycenter: &Barycenter,
    stars: &HashMap<Uuid, &Star>,
    planets: &HashMap<Uuid, &Planet>,
    barycenters: &HashMap<Uuid, &Barycenter>,
) -> DomainResult<Option<Duration>> {
    let (m1, m2) = binary_member_masses(barycenter, stars, planets, barycenters)?;
    let mu = combined_gravitational_parameter(m1, m2);
    let elements = barycenter.internal_orbital_elements();
    Ok(orbital_period(elements.semi_major_axis(), mu))
}

pub fn second_sun_synodic_period(
    planet_orbital_period: Duration,
    binary_orbital_period: Duration,
) -> Option<Duration> {
    synodic_beat_period(planet_orbital_period, binary_orbital_period, true)
}