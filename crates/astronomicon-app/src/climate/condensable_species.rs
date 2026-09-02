use crate::climate::temperature::resolve_global_mean_temperature;
use crate::error::AppResult;
use astronomicon_core::chemistry::solvent::SolventProperties;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::thermodynamics::saturation_vapor_pressure;
use astronomicon_core::units::{Duration, MolarMass, Pressure, Temperature};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{atmosphere_repository, hydrosphere_repository};
use uuid::Uuid;

pub async fn resolve_all_condensable_species(
    pool: &SqlitePool,
    planet_id: Uuid,
) -> AppResult<Vec<(SolventProperties, MolarMass, f64)>> {
    let atmosphere = atmosphere_repository::get_by_planet_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "atmosphere".to_string(),
            reason: format!("planet '{}' has no atmosphere", planet_id),
        })?;

    let ref_temp = resolve_global_mean_temperature(
        pool,
        planet_id,
        Duration::new(0.0),
        Duration::new(0.0),
    )
    .await
    .unwrap_or(Temperature::new(288.15));

    let total_pct: f64 = atmosphere.composition().iter().map(|c| c.percentage()).sum();
    let total_pct = if total_pct > 0.0 { total_pct } else { 100.0 };

    let mut candidates = Vec::new();

    for comp in atmosphere.composition() {
        let formula = comp.formula();
        if let Some(props) = astronomicon_core::chemistry::solvent_properties_of(formula) {
            if let Ok(mm) = astronomicon_core::chemistry::molar_mass_of(formula) {
                let frac = (comp.percentage() / total_pct).clamp(0.0, 1.0);
                let p_partial = Pressure::new(atmosphere.surface_pressure().value() * frac);
                let p_sat = saturation_vapor_pressure(ref_temp, &props);
                let ratio = if p_sat.value() > 0.0 {
                    p_partial.value() / p_sat.value()
                } else {
                    0.0
                };
                candidates.push((props, mm, frac, ratio));
            }
        }
    }

    candidates.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));

    Ok(candidates
        .into_iter()
        .map(|(props, mm, frac, _)| (props, mm, frac))
        .collect())
}

pub async fn resolve_condensable_species(
    pool: &SqlitePool,
    planet_id: Uuid,
) -> AppResult<(SolventProperties, MolarMass, f64)> {
    let atmosphere = atmosphere_repository::get_by_planet_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "atmosphere".to_string(),
            reason: format!("planet '{}' has no atmosphere", planet_id),
        })?;

    let hydro_opt = hydrosphere_repository::get_by_planet_id(pool, &planet_id).await?;

    let (solvent_props, solvent_molar_mass, humidity) = if let Some(hydro) = hydro_opt {
        let props = hydro.mean_solvent_properties()?;
        let mapped: Vec<(String, f64)> = hydro
            .composition()
            .iter()
            .map(|c| (c.formula().to_string(), c.percentage()))
            .collect();
        let mm = astronomicon_core::chemistry::mean_molar_mass(&mapped)
            .unwrap_or_else(|_| MolarMass::new(0.018015));
        let hum = atmosphere
            .surface_humidity()
            .unwrap_or(0.6 * hydro.surface_coverage_fraction().clamp(0.1, 1.0));
        (props, mm, hum)
    } else {
        let all = resolve_all_condensable_species(pool, planet_id).await?;
        let (props, mm) = if let Some(top) = all.into_iter().next() {
            (top.0, top.1)
        } else {
            let default_p = astronomicon_core::chemistry::solvent_properties_of("H2O")
                .expect("H2O solvent properties");
            let default_mm = MolarMass::new(0.018015);
            (default_p, default_mm)
        };
        let hum = atmosphere.surface_humidity().unwrap_or(0.0);
        (props, mm, hum)
    };

    Ok((solvent_props, solvent_molar_mass, humidity))
}
