use astronomicon_core::domain::{Barycenter, BarycenterMember, MinorPlanet, Planet, Star};
use astronomicon_core::error::{DomainError, DomainResult};
use astronomicon_core::math::gravity::{calculate_effective_mass, gravitational_parameter};
use astronomicon_core::math::kepler::orbital_position_secular;
use astronomicon_core::math::perturbation::resolve_secular_precession;
use astronomicon_core::units::{Duration, Mass, Position};
use std::collections::HashMap;
use uuid::Uuid;

pub fn split_barycenter_positions(
    bary_id: Uuid,
    bary_pos: Position,
    barycenter_map: &HashMap<Uuid, &Barycenter>,
    star_map: &HashMap<Uuid, &Star>,
    planet_map: &HashMap<Uuid, &Planet>,
    minor_planet_map: &HashMap<Uuid, &MinorPlanet>,
    memo: &mut HashMap<Uuid, Position>,
    time_since_epoch: Duration,
) -> DomainResult<()> {
    let bary = barycenter_map
        .get(&bary_id)
        .copied()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "barycenter_id".to_string(),
            reason: format!("barycenter '{}' not found", bary_id),
        })?;

    let m_pri = calculate_effective_mass(
        &bary.member_primary(),
        star_map,
        planet_map,
        barycenter_map,
    )?;
    let m_sec = calculate_effective_mass(
        &bary.member_secondary(),
        star_map,
        planet_map,
        barycenter_map,
    )?;

    let m_total = m_pri.value() + m_sec.value();
    let mu_int = gravitational_parameter(Mass::new(m_total));
    let secular_rates =
        resolve_secular_precession(&bary.internal_orbital_elements(), mu_int, None, None);
    let r_rel_int = orbital_position_secular(
        &bary.internal_orbital_elements(),
        mu_int,
        &secular_rates,
        time_since_epoch,
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
            minor_planet_map,
            memo,
            time_since_epoch,
        )?;
    }

    if let BarycenterMember::Barycenter(sub_id) = sec_member {
        split_barycenter_positions(
            sub_id,
            pos_sec,
            barycenter_map,
            star_map,
            planet_map,
            minor_planet_map,
            memo,
            time_since_epoch,
        )?;
    }

    Ok(())
}
