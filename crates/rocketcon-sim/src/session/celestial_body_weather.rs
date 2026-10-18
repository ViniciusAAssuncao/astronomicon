use super::celestial_body_details::CelestialBodyDetails;
use super::celestial_body_facts::{BodyTextFact, push};
use astronomicon_app::climate::atmosphere::calculate_atmospheric_stratification;
use astronomicon_app::climate::circulation::ReferenceWindProfile;
use astronomicon_app::climate::clouds::{
    calculate_cloud_cover, calculate_convective_instability, calculate_tropopause,
};
use astronomicon_app::climate::precipitation::calculate_precipitation_diagnostic_with_shear;
use astronomicon_core::chemistry::solvent::SolventProperties;
use astronomicon_core::chemistry::{mean_molar_mass, molar_mass_of, solvent_properties_of};
use astronomicon_core::domain::{Atmosphere, Planet};
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::precipitation::PrecipitationPhase;
use astronomicon_core::math::thermodynamics::saturation_vapor_pressure;
use astronomicon_core::units::{MolarMass, Temperature};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{atmosphere_repository, hydrosphere_repository};

fn atmospheric_condensable(
    atmosphere: &Atmosphere,
    temperature: Temperature,
) -> Option<(SolventProperties, MolarMass, String)> {
    atmosphere
        .composition()
        .iter()
        .filter_map(|gas| {
            let props = solvent_properties_of(gas.formula())?;
            let molar_mass = molar_mass_of(gas.formula()).ok()?;
            let saturation = saturation_vapor_pressure(temperature, &props).value();
            let fraction = gas.percentage() / 100.0;
            if !saturation.is_finite()
                || saturation <= 0.0
                || !fraction.is_finite()
                || fraction <= 0.0
            {
                return None;
            }
            let score = atmosphere.surface_pressure().value() * fraction / saturation;
            score
                .is_finite()
                .then(|| (props, molar_mass, gas.formula().to_string(), score))
        })
        .max_by(|a, b| a.3.total_cmp(&b.3))
        .map(|(props, molar_mass, formula, _)| (props, molar_mass, formula))
}

pub async fn append_reference_weather(
    details: &mut CelestialBodyDetails,
    pool: &SqlitePool,
    planet: &Planet,
    winds: &ReferenceWindProfile,
) {
    let Some(band) = winds.bands.iter().find(|band| band.latitude_degrees == 30) else {
        return;
    };
    let Some(radius) = planet.equatorial_radius().filter(|r| r.value() > 0.0) else {
        return;
    };
    let Ok(Some(atmosphere)) = atmosphere_repository::get_by_planet_id(pool, &planet.id()).await
    else {
        return;
    };
    let Some(humidity) = atmosphere
        .surface_humidity()
        .filter(|v| v.is_finite() && *v > 0.0 && *v <= 1.0)
    else {
        return;
    };
    let Some(surface_temperature) = (band.temperature_k.is_finite() && band.temperature_k > 0.0)
        .then(|| Temperature::new(band.temperature_k))
    else {
        return;
    };
    if atmosphere.lapse_rate().value() <= 0.0 || !atmosphere.lapse_rate().value().is_finite() {
        return;
    }
    let Ok(cp) = atmosphere.mean_specific_heat_capacity() else {
        return;
    };
    let gravity = surface_gravity(gravitational_parameter(planet.mass()), radius);
    let Ok(height) = atmosphere.scale_height(gravity, surface_temperature) else {
        return;
    };
    let Ok(hydro) = hydrosphere_repository::get_by_planet_id(pool, &planet.id()).await else {
        return;
    };
    let (props, molar_mass, condensable, freezing_point, composition) =
        if let Some(hydro) = hydro.filter(|h| h.surface_coverage_fraction() > 0.0) {
            let Ok(props) = hydro.mean_solvent_properties() else {
                return;
            };
            let components: Vec<_> = hydro
                .composition()
                .iter()
                .map(|c| (c.formula().to_string(), c.percentage()))
                .collect();
            let Ok(molar_mass) = mean_molar_mass(&components) else {
                return;
            };
            let Ok(freezing) = hydro.freezing_point() else {
                return;
            };
            (
                props,
                molar_mass,
                "MISTURA".to_owned(),
                freezing,
                components,
            )
        } else {
            let Some((props, molar_mass, formula)) =
                atmospheric_condensable(&atmosphere, surface_temperature)
            else {
                return;
            };
            let components = vec![(formula.clone(), 100.0)];
            (
                props,
                molar_mass,
                formula,
                props.normal_melting_point,
                components,
            )
        };
    let tropo = calculate_tropopause(
        surface_temperature,
        atmosphere.greenhouse_effect(),
        atmosphere.lapse_rate(),
        gravity,
        cp,
    );
    let Ok(strat) = calculate_atmospheric_stratification(
        &atmosphere,
        planet,
        surface_temperature,
        &props,
        molar_mass,
        humidity,
    ) else {
        return;
    };
    let Ok(instability) =
        calculate_convective_instability(&atmosphere, &tropo, &strat, &props, molar_mass, gravity)
    else {
        return;
    };
    let Ok(clouds) = calculate_cloud_cover(
        &atmosphere,
        &tropo,
        &instability,
        &strat,
        &props,
        molar_mass,
        humidity,
        freezing_point,
        height,
    ) else {
        return;
    };
    let valid_fraction = |fraction: f64| fraction.is_finite() && (0.0..=1.0).contains(&fraction);
    if ![
        clouds.low_cloud.cloud_fraction,
        clouds.mid_cloud.cloud_fraction,
        clouds.high_cloud.cloud_fraction,
    ]
    .iter()
    .all(|v| valid_fraction(*v))
    {
        return;
    }
    push(
        &mut details.facts,
        "weather",
        "latitude_deg",
        Some(band.latitude_degrees as f64),
    );
    let mut add = |key: &str, value: f64| {
        if value.is_finite() && value >= 0.0 {
            push(&mut details.facts, "weather", key, Some(value));
        }
    };
    details.text_facts.push(BodyTextFact {
        section: "weather".into(),
        key: "cloud_condensable".into(),
        value: condensable,
    });
    add("dew_point_k", strat.surface_dew_point.value());
    add("cloud_base_m", strat.lcl_altitude.value());
    add("cloud_top_m", strat.cloud_top_altitude.value());
    add("tropopause_m", tropo.tropopause_altitude.value());
    add("cloud_low_fraction", clouds.low_cloud.cloud_fraction);
    add("cloud_mid_fraction", clouds.mid_cloud.cloud_fraction);
    add("cloud_high_fraction", clouds.high_cloud.cloud_fraction);
    add("convective_energy", instability.cape.value());
    let shear = (band.jet_speed_m_s - band.surface_speed_m_s).abs();
    if let Ok(precipitation) = calculate_precipitation_diagnostic_with_shear(
        &atmosphere,
        &clouds,
        &strat,
        &tropo,
        &instability,
        shear,
        &props,
        molar_mass,
        humidity,
        freezing_point,
        height,
        gravity,
        &composition,
    ) {
        if precipitation.reaches_surface
            && precipitation.mass_flux.value().is_finite()
            && precipitation.mass_flux.value() > 0.0
            && precipitation.linear_accumulation_rate.value().is_finite()
            && precipitation.linear_accumulation_rate.value() > 0.0
        {
            add(
                "precipitation_mm_h",
                precipitation.linear_accumulation_rate.value() * 3_600_000.0,
            );
            let phase = match precipitation.phase {
                PrecipitationPhase::Liquid => "Liquid",
                PrecipitationPhase::Solid => "Solid",
                PrecipitationPhase::Mixed => "Mixed",
            };
            details.text_facts.push(BodyTextFact {
                section: "weather".into(),
                key: "precipitation_phase".into(),
                value: phase.into(),
            });
        }
    }
}
