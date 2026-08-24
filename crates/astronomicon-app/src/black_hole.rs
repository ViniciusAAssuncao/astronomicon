use crate::climate::resolve_stellar_wind_at_distance;
use crate::ephemeris::resolve_system_positions;
use crate::error::AppResult;
use crate::hierarchy::find_companion_star;
use astronomicon_core::domain::{Star, StarKind};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::black_hole::{
    accretion_disk_luminosity, bondi_hoyle_lyttleton_accretion_rate,
    dimensionless_spin_from_rotation_period, eddington_luminosity, event_horizon_radius,
    hawking_luminosity, hawking_temperature, isco_radii, photon_sphere_radii,
    radiative_efficiency,
};
use astronomicon_core::math::gravity::combined_gravitational_parameter;
use astronomicon_core::math::kepler::orbital_speed;
use astronomicon_core::units::{Duration, Length, Luminosity, MassRate, Temperature};
use astronomicon_db::repositories::star_repository;
use astronomicon_db::SqlitePool;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BlackHoleDiagnostic {
    pub dimensionless_spin: f64,
    pub event_horizon_radius: Length,
    pub isco_radius_prograde: Length,
    pub isco_radius_retrograde: Length,
    pub photon_sphere_radius_prograde: Length,
    pub photon_sphere_radius_retrograde: Length,
    pub hawking_temperature: Temperature,
    pub hawking_luminosity: Luminosity,
    pub eddington_luminosity: Luminosity,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AccretionDiagnostic {
    pub accretion_rate: MassRate,
    pub radiative_efficiency: f64,
    pub accretion_luminosity: Luminosity,
    pub hawking_luminosity: Luminosity,
    pub total_luminosity: Luminosity,
    pub eddington_luminosity: Luminosity,
}

pub async fn resolve_black_hole_diagnostics(
    pool: &SqlitePool,
    star_id: Uuid,
) -> AppResult<BlackHoleDiagnostic> {
    let row = star_repository::get_by_id(pool, &star_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "star_id".to_string(),
            reason: format!("star '{}' not found", star_id),
        })?;
    let star = Star::try_from(row)?;

    if star.kind() != StarKind::BlackHole {
        return Err(DomainError::InvalidInvariant {
            field: "kind".to_string(),
            reason: format!("star '{}' is not a black hole", star_id),
        }
        .into());
    }

    let spin = star
        .rotation_period()
        .map(|p| dimensionless_spin_from_rotation_period(star.mass(), p))
        .unwrap_or(0.0);

    let r_eh = event_horizon_radius(star.mass(), spin);
    let (r_isco_pro, r_isco_ret) = isco_radii(star.mass(), spin);
    let (r_ph_pro, r_ph_ret) = photon_sphere_radii(star.mass(), spin);
    let t_h = hawking_temperature(star.mass(), spin);
    let l_h = hawking_luminosity(star.mass(), spin);
    let l_edd = eddington_luminosity(star.mass());

    Ok(BlackHoleDiagnostic {
        dimensionless_spin: spin,
        event_horizon_radius: r_eh,
        isco_radius_prograde: r_isco_pro,
        isco_radius_retrograde: r_isco_ret,
        photon_sphere_radius_prograde: r_ph_pro,
        photon_sphere_radius_retrograde: r_ph_ret,
        hawking_temperature: t_h,
        hawking_luminosity: l_h,
        eddington_luminosity: l_edd,
    })
}

pub async fn resolve_black_hole_accretion(
    pool: &SqlitePool,
    star_id: Uuid,
    eta: f64,
    wind_scaling: f64,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<AccretionDiagnostic> {
    let row = star_repository::get_by_id(pool, &star_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "star_id".to_string(),
            reason: format!("star '{}' not found", star_id),
        })?;
    let star = Star::try_from(row)?;

    if star.kind() != StarKind::BlackHole {
        return Err(DomainError::InvalidInvariant {
            field: "kind".to_string(),
            reason: format!("star '{}' is not a black hole", star_id),
        }
        .into());
    }

    let spin = star
        .rotation_period()
        .map(|p| dimensionless_spin_from_rotation_period(star.mass(), p))
        .unwrap_or(0.0);

    let l_h = hawking_luminosity(star.mass(), spin);
    let l_edd = eddington_luminosity(star.mass());
    let eta_rad = radiative_efficiency(spin);

    let companion_opt = find_companion_star(pool, &star).await?;

    match companion_opt {
        Some(companion) => {
            let comp_temp = companion.effective_temperature();
            let comp_radius = companion.radius();
            let system_id = star.star_system_id().or_else(|| companion.star_system_id());

            match (comp_temp, comp_radius, system_id) {
                (Some(t_comp), Some(r_comp), Some(sys_id)) => {
                    let total_epoch = universe_epoch + at_epoch;
                    let positions = resolve_system_positions(pool, sys_id, total_epoch).await?;

                    if let (Some(&bh_pos), Some(&comp_pos)) = (
                        positions.get(&star.id()),
                        positions.get(&companion.id()),
                    ) {
                        let orbital_dist = (bh_pos - comp_pos).magnitude();
                        let wind_diag = resolve_stellar_wind_at_distance(
                            companion.mass(),
                            r_comp,
                            t_comp,
                            orbital_dist,
                            eta,
                            wind_scaling,
                        );

                        let mu_total =
                            combined_gravitational_parameter(star.mass(), companion.mass());
                        let semi_major = star
                            .orbital_elements()
                            .map(|e| e.semi_major_axis())
                            .or_else(|| companion.orbital_elements().map(|e| e.semi_major_axis()))
                            .unwrap_or(orbital_dist);

                        let v_orb = orbital_speed(mu_total, orbital_dist, semi_major);

                        let m_dot = bondi_hoyle_lyttleton_accretion_rate(
                            star.mass(),
                            wind_diag.wind_density_at_orbit,
                            wind_diag.terminal_wind_speed,
                            v_orb,
                        );

                        let l_acc = accretion_disk_luminosity(m_dot, eta_rad, star.mass());
                        let total_l = Luminosity::new(l_acc.value() + l_h.value());

                        Ok(AccretionDiagnostic {
                            accretion_rate: m_dot,
                            radiative_efficiency: eta_rad,
                            accretion_luminosity: l_acc,
                            hawking_luminosity: l_h,
                            total_luminosity: total_l,
                            eddington_luminosity: l_edd,
                        })
                    } else {
                        Ok(AccretionDiagnostic {
                            accretion_rate: MassRate::new(0.0),
                            radiative_efficiency: eta_rad,
                            accretion_luminosity: Luminosity::new(0.0),
                            hawking_luminosity: l_h,
                            total_luminosity: l_h,
                            eddington_luminosity: l_edd,
                        })
                    }
                }
                _ => Ok(AccretionDiagnostic {
                    accretion_rate: MassRate::new(0.0),
                    radiative_efficiency: eta_rad,
                    accretion_luminosity: Luminosity::new(0.0),
                    hawking_luminosity: l_h,
                    total_luminosity: l_h,
                    eddington_luminosity: l_edd,
                }),
            }
        }
        None => Ok(AccretionDiagnostic {
            accretion_rate: MassRate::new(0.0),
            radiative_efficiency: eta_rad,
            accretion_luminosity: Luminosity::new(0.0),
            hawking_luminosity: l_h,
            total_luminosity: l_h,
            eddington_luminosity: l_edd,
        }),
    }
}
