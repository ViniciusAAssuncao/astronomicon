use super::celestial_body_details::CelestialBodyDetails;
use super::celestial_body_facts::{BodyTextFact, push};
use astronomicon_app::geology::resolve_planetary_geology;
use astronomicon_app::geophysics::resolve_planetary_core;
use astronomicon_app::tidal::resolve_tidal_diagnostics;
use astronomicon_core::domain::PlanetKind;
use astronomicon_core::units::Duration;
use astronomicon_db::SqlitePool;
use uuid::Uuid;

pub async fn append_geoscience(
    details: &mut CelestialBodyDetails,
    pool: &SqlitePool,
    body_id: Uuid,
    kind: PlanetKind,
    universe_epoch: Duration,
    at_epoch: Duration,
) {
    if let Ok(core) = resolve_planetary_core(pool, body_id, universe_epoch, at_epoch).await {
        if core.core_radius.value() > 0.0 {
            push(
                &mut details.facts,
                "interior",
                "core_radius_m",
                Some(core.core_radius.value()),
            );
            push(
                &mut details.facts,
                "interior",
                "core_density",
                Some(core.core_density.value()),
            );
            push(
                &mut details.facts,
                "interior",
                "core_heat_flux",
                Some(core.cmb_heat_flux.value()),
            );
            push(
                &mut details.facts,
                "interior",
                "core_convection",
                Some(core.convective_heat_flux.value()),
            );
            push(
                &mut details.facts,
                "interior",
                "radiogenic_flux",
                Some(core.radiogenic_heat_flux.value()),
            );
            push(
                &mut details.facts,
                "interior",
                "geothermal_flux",
                Some(core.total_surface_heat_flux.value()),
            );
        }
    }
    if let Ok(tidal) = resolve_tidal_diagnostics(pool, body_id, universe_epoch, at_epoch).await {
        let energy = tidal.tidal_heating_energy.value();
        if energy > 0.0 {
            push(&mut details.facts, "tides", "tidal_power_w", Some(energy));
            push(
                &mut details.facts,
                "tides",
                "tidal_flux",
                Some(tidal.tidal_surface_heat_flux.value()),
            );
        }
        if tidal.tidal_locking_timescale.value() > 0.0 {
            push(
                &mut details.facts,
                "tides",
                "tidal_lock_seconds",
                Some(tidal.tidal_locking_timescale.value()),
            );
        }
    }
    if matches!(kind, PlanetKind::GasGiant | PlanetKind::IceGiant) {
        return;
    }
    if let Ok(geo) = resolve_planetary_geology(pool, body_id, universe_epoch, at_epoch).await {
        details.text_facts.push(BodyTextFact {
            section: "tectonics".into(),
            key: "tectonic_regime".into(),
            value: format!("{:?}", geo.tectonic_regime),
        });
        push(
            &mut details.facts,
            "tectonics",
            "lithosphere_thickness",
            Some(geo.lithosphere_thickness.value()),
        );
        if geo.plate_count > 0 {
            push(
                &mut details.facts,
                "tectonics",
                "plate_count",
                Some(f64::from(geo.plate_count)),
            );
        }
        if geo.plate_velocity.value() > 0.0 {
            push(
                &mut details.facts,
                "tectonics",
                "plate_velocity",
                Some(geo.plate_velocity.value()),
            );
        }
        if geo.native_seismic_energy.value() > 0.0 {
            push(
                &mut details.facts,
                "tectonics",
                "tectonic_seismic_power",
                Some(geo.native_seismic_energy.value()),
            );
        }
        if geo.tidal_seismic_energy.value() > 0.0 {
            push(
                &mut details.facts,
                "tectonics",
                "tidal_seismic_power",
                Some(geo.tidal_seismic_energy.value()),
            );
        }
    }
}
