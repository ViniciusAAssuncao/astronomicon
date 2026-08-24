use crate::error::AppResult;
use astronomicon_core::domain::{ Star, StarKind };
use astronomicon_core::error::DomainError;
use astronomicon_core::math::black_hole::{
    dimensionless_spin_from_rotation_period,
    eddington_luminosity,
    event_horizon_radius,
    hawking_luminosity,
    hawking_temperature,
    isco_radii,
    photon_sphere_radii,
};
use astronomicon_core::units::{ Length, Luminosity, Temperature };
use astronomicon_db::repositories::star_repository;
use astronomicon_db::SqlitePool;
use serde::{ Deserialize, Serialize };
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

pub async fn resolve_black_hole_diagnostics(
    pool: &SqlitePool,
    star_id: Uuid
) -> AppResult<BlackHoleDiagnostic> {
    let row = star_repository
        ::get_by_id(pool, &star_id).await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "star_id".to_string(),
            reason: format!("star '{}' not found", star_id),
        })?;
    let star = Star::try_from(row)?;

    if star.kind() != StarKind::BlackHole {
        return Err(
            (DomainError::InvalidInvariant {
                field: "kind".to_string(),
                reason: format!("star '{}' is not a black hole", star_id),
            }).into()
        );
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
