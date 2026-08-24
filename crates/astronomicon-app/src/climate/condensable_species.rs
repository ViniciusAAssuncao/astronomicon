use crate::error::AppResult;
use astronomicon_core::chemistry::solvent::SolventProperties;
use astronomicon_core::error::DomainError;
use astronomicon_core::units::MolarMass;
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{atmosphere_repository, hydrosphere_repository};
use uuid::Uuid;

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
        let found = atmosphere.composition().iter().find_map(|c| {
            let formula = c.formula();
            astronomicon_core::chemistry::solvent_properties_of(formula).and_then(|p| {
                astronomicon_core::chemistry::molar_mass_of(formula)
                    .ok()
                    .map(|mm| (p, mm))
            })
        });

        let (props, mm) = match found {
            Some((p, mm)) => (p, mm),
            None => {
                let default_p = astronomicon_core::chemistry::solvent_properties_of("H2O")
                    .expect("H2O solvent properties");
                let default_mm = MolarMass::new(0.018015);
                (default_p, default_mm)
            }
        };
        let hum = atmosphere.surface_humidity().unwrap_or(0.0);
        (props, mm, hum)
    };

    Ok((solvent_props, solvent_molar_mass, humidity))
}
