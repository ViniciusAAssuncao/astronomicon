use crate::constants::constants::SYNCHRONOUS_ROTATION_TOLERANCE;
use crate::domain::day_analysis::{is_retrograde_obliquity, is_tidally_synchronized};
use crate::domain::rotation_classification::RotationClassification;
use crate::math::day_length::solar_day_length;
use crate::math::multi_star::binary::binary_orbital_period;
use astronomicon_core::domain::{
    Barycenter, BarycenterMember, MinorPlanet, OrbitalParent, Planet, Star,
};
use astronomicon_core::math::gravity::{
    calculate_effective_mass, combined_gravitational_parameter,
};
use astronomicon_core::math::kepler::orbital_period;
use astronomicon_core::units::Duration;
use std::collections::HashMap;
use uuid::Uuid;

fn barycenter_contains_star(
    bary_id: Uuid,
    target_star_id: Uuid,
    barycenters: &HashMap<Uuid, &Barycenter>,
) -> bool {
    if let Some(bary) = barycenters.get(&bary_id) {
        let check_member = |member: &BarycenterMember| -> bool {
            match member {
                BarycenterMember::Star(id) => *id == target_star_id,
                BarycenterMember::Barycenter(sub_id) => {
                    barycenter_contains_star(*sub_id, target_star_id, barycenters)
                }
                BarycenterMember::Planet(_) => false,
            }
        };
        check_member(&bary.member_primary()) || check_member(&bary.member_secondary())
    } else {
        false
    }
}

pub fn star_effective_orbital_period(
    planet: &Planet,
    star_id: Uuid,
    stars: &HashMap<Uuid, &Star>,
    planets: &HashMap<Uuid, &Planet>,
    barycenters: &HashMap<Uuid, &Barycenter>,
    _minor_planets: &HashMap<Uuid, &MinorPlanet>,
) -> Option<Duration> {
    let parent = planet.orbital_parent();
    match parent {
        OrbitalParent::Star(p_star_id) => {
            if p_star_id == star_id {
                let elements = planet.orbital_elements()?;
                let star = stars.get(&star_id)?;
                let mu = combined_gravitational_parameter(planet.mass(), star.mass());
                orbital_period(elements.semi_major_axis(), mu)
            } else {
                for bary in barycenters.values() {
                    let has_parent = bary.member_primary().id() == p_star_id
                        || bary.member_secondary().id() == p_star_id;
                    let has_target = bary.member_primary().id() == star_id
                        || bary.member_secondary().id() == star_id
                        || barycenter_contains_star(bary.id(), star_id, barycenters);
                    if has_parent && has_target {
                        if let Ok(Some(period)) =
                            binary_orbital_period(bary, stars, planets, barycenters)
                        {
                            return Some(period);
                        }
                    }
                }
                None
            }
        }
        OrbitalParent::Barycenter(bary_id) => {
            if barycenter_contains_star(bary_id, star_id, barycenters) {
                let elements = planet.orbital_elements()?;
                let m_bary = calculate_effective_mass(
                    &BarycenterMember::Barycenter(bary_id),
                    stars,
                    planets,
                    barycenters,
                )
                .ok()?;
                let mu = combined_gravitational_parameter(planet.mass(), m_bary);
                orbital_period(elements.semi_major_axis(), mu)
            } else {
                let bary = barycenters.get(&bary_id)?;
                let mut curr_parent = bary.orbital_parent();
                while let OrbitalParent::Barycenter(ancestor_id) = curr_parent {
                    if barycenter_contains_star(ancestor_id, star_id, barycenters) {
                        if let Some(anc_bary) = barycenters.get(&ancestor_id) {
                            if let Ok(Some(period)) =
                                binary_orbital_period(anc_bary, stars, planets, barycenters)
                            {
                                return Some(period);
                            }
                        }
                    }
                    if let Some(anc_bary) = barycenters.get(&ancestor_id) {
                        curr_parent = anc_bary.orbital_parent();
                    } else {
                        break;
                    }
                }
                None
            }
        }
        _ => None,
    }
}

pub fn star_individual_solar_day(
    planet: &Planet,
    star_id: Uuid,
    stars: &HashMap<Uuid, &Star>,
    planets: &HashMap<Uuid, &Planet>,
    barycenters: &HashMap<Uuid, &Barycenter>,
    minor_planets: &HashMap<Uuid, &MinorPlanet>,
) -> (Option<Duration>, RotationClassification) {
    let t_rot = match planet.rotation_period() {
        Some(rot) if rot.value() > 0.0 && rot.value().is_finite() => rot,
        _ => return (None, RotationClassification::NoRotationalData),
    };

    let is_retrograde = is_retrograde_obliquity(planet.obliquity());
    let t_orb = star_effective_orbital_period(
        planet,
        star_id,
        stars,
        planets,
        barycenters,
        minor_planets,
    );

    if let Some(orb) = t_orb {
        if !is_retrograde && is_tidally_synchronized(t_rot, orb, SYNCHRONOUS_ROTATION_TOLERANCE) {
            return (None, RotationClassification::Synchronous);
        }
    }

    let classification = if is_retrograde {
        RotationClassification::Retrograde
    } else {
        RotationClassification::Prograde
    };

    let t_solar = match t_orb {
        Some(orb) => solar_day_length(t_rot, orb, is_retrograde),
        None => Some(t_rot),
    };

    (t_solar, classification)
}