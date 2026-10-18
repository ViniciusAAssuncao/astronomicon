use super::celestial_body_details::CelestialBodyDetails;
use super::celestial_body_facts::{BodyTextFact, push};
use astronomicon_app::volcanism::resolve_planetary_volcanism;
use astronomicon_core::domain::PlanetKind;
use astronomicon_core::units::Duration;
use astronomicon_db::SqlitePool;
use uuid::Uuid;

pub async fn append_volcanism(
    details: &mut CelestialBodyDetails,
    pool: &SqlitePool,
    id: Uuid,
    kind: PlanetKind,
    universe_epoch: Duration,
    at_epoch: Duration,
) {
    if matches!(kind, PlanetKind::GasGiant | PlanetKind::IceGiant) {
        return;
    }
    let Ok(model) = resolve_planetary_volcanism(pool, id, universe_epoch, at_epoch).await else {
        return;
    };
    let rate = model.global_magma_production_rate.value();
    if !rate.is_finite() || rate <= 0.0 {
        return;
    }
    let style = if model.is_cryovolcanic {
        "Cryovolcanic"
    } else if model.explosive_fraction > model.effusive_fraction {
        "Explosive"
    } else {
        "Effusive"
    };
    details.text_facts.push(BodyTextFact {
        section: "volcanism".into(),
        key: "volcanic_style".into(),
        value: style.into(),
    });
    if model.is_magma_ocean {
        details.text_facts.push(BodyTextFact {
            section: "volcanism".into(),
            key: "magma_ocean".into(),
            value: "Present".into(),
        });
    }
    let mut add = |key: &str, value: f64| {
        if value.is_finite() && value > 0.0 {
            push(&mut details.facts, "volcanism", key, Some(value));
        }
    };
    add("magma_production_rate", rate);
    add("magma_temperature_k", model.magma_temperature.value());
    add("magma_viscosity_pa_s", model.magma_viscosity.value());
    add("effusive_fraction", model.effusive_fraction);
    add("explosive_fraction", model.explosive_fraction);
    add("outgassing_h2o", model.outgassing_rate_h2o.value());
    add("outgassing_co2", model.outgassing_rate_co2.value());
    add("outgassing_sulfur", model.outgassing_rate_sulfur.value());
}
