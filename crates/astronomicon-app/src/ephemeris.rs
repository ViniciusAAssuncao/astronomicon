use crate::error::AppResult;
use astronomicon_core::domain::{ Barycenter, BarycenterMember, OrbitalParent, Planet, Star };
use astronomicon_core::error::{ DomainError, DomainResult };
use astronomicon_core::math::gravity::{
    calculate_effective_mass,
    calculate_parent_effective_mass,
    combined_gravitational_parameter,
    gravitational_parameter,
};
use astronomicon_core::math::kepler::orbital_position_secular;
use astronomicon_core::math::perturbation::resolve_secular_precession;
use astronomicon_core::units::{ Duration, Length, Mass, Position };
use astronomicon_db::SqlitePool;
use std::collections::{ HashMap, HashSet };
use uuid::Uuid;

fn get_parent_j2_and_radius(
    parent: &OrbitalParent,
    star_map: &HashMap<Uuid, &Star>,
    planet_map: &HashMap<Uuid, &Planet>
) -> (Option<f64>, Option<Length>) {
    match parent {
        OrbitalParent::Star(pid) => {
            let p = star_map.get(pid).copied();
            (p.and_then(|s| s.oblateness_j2()), p.and_then(|s| s.radius()))
        }
        OrbitalParent::Planet(pid) => {
            let p = planet_map.get(pid).copied();
            (p.and_then(|pl| pl.oblateness_j2()), p.and_then(|pl| pl.equatorial_radius()))
        }
        OrbitalParent::Barycenter(_) | OrbitalParent::Fixed => (None, None),
    }
}

fn split_barycenter_positions(
    bary_id: Uuid,
    bary_pos: Position,
    barycenter_map: &HashMap<Uuid, &Barycenter>,
    star_map: &HashMap<Uuid, &Star>,
    planet_map: &HashMap<Uuid, &Planet>,
    memo: &mut HashMap<Uuid, Position>,
    time_since_epoch: Duration
) -> DomainResult<()> {
    let bary = barycenter_map
        .get(&bary_id)
        .copied()
        .ok_or_else(|| {
            DomainError::InvalidInvariant {
                field: "barycenter_id".to_string(),
                reason: format!("barycenter '{}' not found", bary_id),
            }
        })?;

    let m_pri = calculate_effective_mass(
        &bary.member_primary(),
        star_map,
        planet_map,
        barycenter_map
    )?;
    let m_sec = calculate_effective_mass(
        &bary.member_secondary(),
        star_map,
        planet_map,
        barycenter_map
    )?;

    let m_total = m_pri.value() + m_sec.value();
    let mu_int = gravitational_parameter(Mass::new(m_total));
    let secular_rates = resolve_secular_precession(
        &bary.internal_orbital_elements(),
        mu_int,
        None,
        None
    );
    let r_rel_int = orbital_position_secular(
        &bary.internal_orbital_elements(),
        mu_int,
        &secular_rates,
        time_since_epoch
    )?;

    let (frac_pri, frac_sec) = if m_total > 0.0 {
        (m_pri.value() / m_total, m_sec.value() / m_total)
    } else {
        (0.5, 0.5)
    };

    let pos_sec = bary_pos + r_rel_int * frac_pri;
    let pos_pri = bary_pos - r_rel_int * frac_sec;

    let pri_member = bary.member_primary();
    let sec_member = bary.member_secondary();

    memo.insert(pri_member.id(), pos_pri);
    memo.insert(sec_member.id(), pos_sec);

    if let BarycenterMember::Barycenter(sub_id) = pri_member {
        split_barycenter_positions(
            sub_id,
            pos_pri,
            barycenter_map,
            star_map,
            planet_map,
            memo,
            time_since_epoch
        )?;
    }

    if let BarycenterMember::Barycenter(sub_id) = sec_member {
        split_barycenter_positions(
            sub_id,
            pos_sec,
            barycenter_map,
            star_map,
            planet_map,
            memo,
            time_since_epoch
        )?;
    }

    Ok(())
}

fn resolve_node(
    id: Uuid,
    member_of: &HashMap<Uuid, Uuid>,
    star_map: &HashMap<Uuid, &Star>,
    planet_map: &HashMap<Uuid, &Planet>,
    barycenter_map: &HashMap<Uuid, &Barycenter>,
    memo: &mut HashMap<Uuid, Position>,
    visiting: &mut HashSet<Uuid>,
    time_since_epoch: Duration
) -> DomainResult<Position> {
    if let Some(&pos) = memo.get(&id) {
        return Ok(pos);
    }

    if !visiting.insert(id) {
        return Err(DomainError::InvalidInvariant {
            field: "orbital_hierarchy".to_string(),
            reason: format!("circular dependency detected for entity '{}'", id),
        });
    }

    if let Some(&parent_bary_id) = member_of.get(&id) {
        let bary_pos = resolve_node(
            parent_bary_id,
            member_of,
            star_map,
            planet_map,
            barycenter_map,
            memo,
            visiting,
            time_since_epoch
        )?;

        split_barycenter_positions(
            parent_bary_id,
            bary_pos,
            barycenter_map,
            star_map,
            planet_map,
            memo,
            time_since_epoch
        )?;

        visiting.remove(&id);

        return memo
            .get(&id)
            .copied()
            .ok_or_else(|| {
                DomainError::InvalidInvariant {
                    field: "barycenter_member".to_string(),
                    reason: format!(
                        "position for member '{}' not resolved by barycenter '{}'",
                        id,
                        parent_bary_id
                    ),
                }
            });
    }

    let pos = if let Some(star) = star_map.get(&id).copied() {
        match star.orbital_parent() {
            OrbitalParent::Fixed => Position::zero(),
            parent => {
                let parent_id = match parent {
                    | OrbitalParent::Star(pid)
                    | OrbitalParent::Planet(pid)
                    | OrbitalParent::Barycenter(pid) => pid,
                    OrbitalParent::Fixed => unreachable!(),
                };
                let parent_pos = resolve_node(
                    parent_id,
                    member_of,
                    star_map,
                    planet_map,
                    barycenter_map,
                    memo,
                    visiting,
                    time_since_epoch
                )?;
                let elements = star.orbital_elements().ok_or_else(|| {
                    DomainError::InvalidInvariant {
                        field: "orbital_elements".to_string(),
                        reason: format!("star '{}' has orbital parent but no orbital elements", id),
                    }
                })?;
                let m_node = star.mass();
                let m_parent = calculate_parent_effective_mass(
                    &parent,
                    star_map,
                    planet_map,
                    barycenter_map
                )?;
                let mu = combined_gravitational_parameter(m_node, m_parent);
                let (parent_j2, parent_radius) = get_parent_j2_and_radius(
                    &parent,
                    star_map,
                    planet_map
                );
                let secular_rates = resolve_secular_precession(
                    &elements,
                    mu,
                    parent_j2,
                    parent_radius
                );
                let rel_pos = orbital_position_secular(
                    &elements,
                    mu,
                    &secular_rates,
                    time_since_epoch
                )?;
                parent_pos + rel_pos
            }
        }
    } else if let Some(planet) = planet_map.get(&id).copied() {
        match planet.orbital_parent() {
            OrbitalParent::Fixed => Position::zero(),
            parent => {
                let parent_id = match parent {
                    | OrbitalParent::Star(pid)
                    | OrbitalParent::Planet(pid)
                    | OrbitalParent::Barycenter(pid) => pid,
                    OrbitalParent::Fixed => unreachable!(),
                };
                let parent_pos = resolve_node(
                    parent_id,
                    member_of,
                    star_map,
                    planet_map,
                    barycenter_map,
                    memo,
                    visiting,
                    time_since_epoch
                )?;
                let elements = planet.orbital_elements().ok_or_else(|| {
                    DomainError::InvalidInvariant {
                        field: "orbital_elements".to_string(),
                        reason: format!("planet '{}' has orbital parent but no orbital elements", id),
                    }
                })?;
                let m_node = planet.mass();
                let m_parent = calculate_parent_effective_mass(
                    &parent,
                    star_map,
                    planet_map,
                    barycenter_map
                )?;
                let mu = combined_gravitational_parameter(m_node, m_parent);
                let (parent_j2, parent_radius) = get_parent_j2_and_radius(
                    &parent,
                    star_map,
                    planet_map
                );
                let secular_rates = resolve_secular_precession(
                    &elements,
                    mu,
                    parent_j2,
                    parent_radius
                );
                let rel_pos = orbital_position_secular(
                    &elements,
                    mu,
                    &secular_rates,
                    time_since_epoch
                )?;
                parent_pos + rel_pos
            }
        }
    } else if let Some(bary) = barycenter_map.get(&id).copied() {
        let bary_pos = match bary.orbital_parent() {
            OrbitalParent::Fixed => Position::zero(),
            parent => {
                let parent_id = match parent {
                    | OrbitalParent::Star(pid)
                    | OrbitalParent::Planet(pid)
                    | OrbitalParent::Barycenter(pid) => pid,
                    OrbitalParent::Fixed => unreachable!(),
                };
                let parent_pos = resolve_node(
                    parent_id,
                    member_of,
                    star_map,
                    planet_map,
                    barycenter_map,
                    memo,
                    visiting,
                    time_since_epoch
                )?;
                let elements = bary.external_orbital_elements().ok_or_else(|| {
                    DomainError::InvalidInvariant {
                        field: "external_orbital_elements".to_string(),
                        reason: format!("barycenter '{}' has orbital parent but no external orbital elements", id),
                    }
                })?;
                let m_node = calculate_effective_mass(
                    &BarycenterMember::Barycenter(id),
                    star_map,
                    planet_map,
                    barycenter_map
                )?;
                let m_parent = calculate_parent_effective_mass(
                    &parent,
                    star_map,
                    planet_map,
                    barycenter_map
                )?;
                let mu = combined_gravitational_parameter(m_node, m_parent);
                let (parent_j2, parent_radius) = get_parent_j2_and_radius(
                    &parent,
                    star_map,
                    planet_map
                );
                let secular_rates = resolve_secular_precession(
                    &elements,
                    mu,
                    parent_j2,
                    parent_radius
                );
                let rel_pos = orbital_position_secular(
                    &elements,
                    mu,
                    &secular_rates,
                    time_since_epoch
                )?;
                parent_pos + rel_pos
            }
        };

        memo.insert(id, bary_pos);

        split_barycenter_positions(
            id,
            bary_pos,
            barycenter_map,
            star_map,
            planet_map,
            memo,
            time_since_epoch
        )?;

        bary_pos
    } else {
        return Err(DomainError::InvalidInvariant {
            field: "entity_id".to_string(),
            reason: format!("entity '{}' not found in system hierarchy", id),
        });
    };

    memo.insert(id, pos);
    visiting.remove(&id);
    Ok(pos)
}

pub fn compute_system_positions(
    stars: &[Star],
    planets: &[Planet],
    barycenters: &[Barycenter],
    time_since_epoch: Duration
) -> DomainResult<HashMap<Uuid, Position>> {
    let star_map: HashMap<Uuid, &Star> = stars
        .iter()
        .map(|s| (s.id(), s))
        .collect();
    let planet_map: HashMap<Uuid, &Planet> = planets
        .iter()
        .map(|p| (p.id(), p))
        .collect();
    let barycenter_map: HashMap<Uuid, &Barycenter> = barycenters
        .iter()
        .map(|b| (b.id(), b))
        .collect();

    let mut member_of: HashMap<Uuid, Uuid> = HashMap::with_capacity(barycenters.len() * 2);
    for b in barycenters {
        let pri_id = b.member_primary().id();
        let sec_id = b.member_secondary().id();

        if member_of.insert(pri_id, b.id()).is_some() {
            return Err(DomainError::InvalidInvariant {
                field: "barycenters".to_string(),
                reason: format!("entity '{}' is a member of multiple barycenters", pri_id),
            });
        }
        if member_of.insert(sec_id, b.id()).is_some() {
            return Err(DomainError::InvalidInvariant {
                field: "barycenters".to_string(),
                reason: format!("entity '{}' is a member of multiple barycenters", sec_id),
            });
        }
    }

    let mut memo: HashMap<Uuid, Position> = HashMap::with_capacity(
        stars.len() + planets.len() + barycenters.len()
    );
    let mut visiting: HashSet<Uuid> = HashSet::new();

    for barycenter in barycenters {
        resolve_node(
            barycenter.id(),
            &member_of,
            &star_map,
            &planet_map,
            &barycenter_map,
            &mut memo,
            &mut visiting,
            time_since_epoch
        )?;
    }

    for star in stars {
        resolve_node(
            star.id(),
            &member_of,
            &star_map,
            &planet_map,
            &barycenter_map,
            &mut memo,
            &mut visiting,
            time_since_epoch
        )?;
    }

    for planet in planets {
        resolve_node(
            planet.id(),
            &member_of,
            &star_map,
            &planet_map,
            &barycenter_map,
            &mut memo,
            &mut visiting,
            time_since_epoch
        )?;
    }

    Ok(memo)
}

pub async fn resolve_system_positions(
    pool: &SqlitePool,
    star_system_id: Uuid,
    time_since_epoch: Duration
) -> AppResult<HashMap<Uuid, Position>> {
    let star_rows = astronomicon_db::repositories::star_repository::list_by_system(
        pool,
        &star_system_id
    ).await?;
    let stars = star_rows.into_iter().map(Star::try_from).collect::<Result<Vec<_>, _>>()?;

    let planet_rows = astronomicon_db::repositories::planet_repository::list_by_system(
        pool,
        &star_system_id
    ).await?;
    let planets = planet_rows.into_iter().map(Planet::try_from).collect::<Result<Vec<_>, _>>()?;

    let barycenter_rows = astronomicon_db::repositories::barycenter_repository::list_by_system(
        pool,
        &star_system_id
    ).await?;
    let barycenters = barycenter_rows
        .into_iter()
        .map(Barycenter::try_from)
        .collect::<Result<Vec<_>, _>>()?;

    let positions = compute_system_positions(&stars, &planets, &barycenters, time_since_epoch)?;
    Ok(positions)
}
