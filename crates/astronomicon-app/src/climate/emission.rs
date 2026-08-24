use crate::black_hole::{resolve_black_hole_accretion, resolve_black_hole_diagnostics};
use crate::error::AppResult;
use astronomicon_core::domain::{Star, StarKind};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::radiometry::stellar_luminosity;
use astronomicon_core::units::constants::STEFAN_BOLTZMANN_CONSTANT;
use astronomicon_core::units::{Duration, Length, Luminosity, Temperature};
use astronomicon_db::SqlitePool;
use std::f64::consts::PI;

pub async fn resolve_star_emission_profile(
    pool: &SqlitePool,
    star: &Star,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<(Luminosity, Temperature, Length)> {
    if star.kind() == StarKind::BlackHole {
        let acc = resolve_black_hole_accretion(
            pool,
            star.id(),
            1.0,
            1.0,
            universe_epoch,
            at_epoch,
        )
        .await?;
        let bh_diag = resolve_black_hole_diagnostics(pool, star.id()).await?;
        let r_emit = bh_diag.isco_radius_prograde;

        let eff_temp = if acc.accretion_luminosity.value() > 0.0 {
            let area = 4.0 * PI * r_emit.value() * r_emit.value();
            if area > 0.0 {
                let t4 = acc.accretion_luminosity.value() / (area * STEFAN_BOLTZMANN_CONSTANT);
                Temperature::new(t4.max(0.0).powf(0.25))
            } else {
                bh_diag.hawking_temperature
            }
        } else {
            bh_diag.hawking_temperature
        };

        Ok((acc.total_luminosity, eff_temp, r_emit))
    } else {
        let star_temp = star
            .effective_temperature()
            .ok_or_else(|| DomainError::InvalidInvariant {
                field: "effective_temperature".to_string(),
                reason: "star does not have effective temperature".to_string(),
            })?;
        let star_radius = star.radius().ok_or_else(|| DomainError::InvalidInvariant {
            field: "radius".to_string(),
            reason: "star does not have radius".to_string(),
        })?;
        let star_lum = stellar_luminosity(star_radius, star_temp);
        Ok((star_lum, star_temp, star_radius))
    }
}
