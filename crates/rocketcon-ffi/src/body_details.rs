use crate::ffi::{FfiBodyComponent, FfiBodyDetails, FfiBodyFact, FfiBodyMaterial, FfiBodyTextFact};
use rocketcon_sim::{BodyComposition, CelestialBodyDetails};

fn components(values: Vec<BodyComposition>) -> Vec<FfiBodyComponent> {
    values
        .into_iter()
        .map(|item| FfiBodyComponent {
            formula: item.formula,
            percentage: item.percentage,
        })
        .collect()
}

impl From<CelestialBodyDetails> for FfiBodyDetails {
    fn from(value: CelestialBodyDetails) -> Self {
        let atmosphere = value.atmosphere;
        let hydrosphere = value.hydrosphere;
        let has_atmosphere = atmosphere.is_some();
        let has_hydrosphere = hydrosphere.is_some();
        let (pressure_pa, gases) = atmosphere.map_or((0.0, Vec::new()), |item| {
            (item.pressure_pa, components(item.gases))
        });
        let (coverage_percent, depth_m, liquids) =
            hydrosphere.map_or((0.0, 0.0, Vec::new()), |item| {
                (
                    item.coverage_percent,
                    item.depth_m,
                    components(item.components),
                )
            });
        Self {
            id: value.id.to_string(),
            has_gravity: value.gravity_m_s2.is_some(),
            gravity_m_s2: value.gravity_m_s2.unwrap_or(0.0),
            has_density: value.density_kg_m3.is_some(),
            density_kg_m3: value.density_kg_m3.unwrap_or(0.0),
            has_rotation: value.rotation_seconds.is_some(),
            rotation_seconds: value.rotation_seconds.unwrap_or(0.0),
            has_distance: value.orbital_distance_m.is_some(),
            orbital_distance_m: value.orbital_distance_m.unwrap_or(0.0),
            has_period: value.orbital_period_seconds.is_some(),
            orbital_period_seconds: value.orbital_period_seconds.unwrap_or(0.0),
            has_eccentricity: value.eccentricity.is_some(),
            eccentricity: value.eccentricity.unwrap_or(0.0),
            has_inclination: value.inclination_deg.is_some(),
            inclination_deg: value.inclination_deg.unwrap_or(0.0),
            has_temperature: value.temperature_k.is_some(),
            temperature_k: value.temperature_k.unwrap_or(0.0),
            reference_radius_gravity: value.reference_radius_gravity,
            has_atmosphere,
            pressure_pa,
            gases,
            has_hydrosphere,
            coverage_percent,
            depth_m,
            liquids,
            facts: value
                .facts
                .into_iter()
                .map(|fact| FfiBodyFact {
                    section: fact.section,
                    key: fact.key,
                    value: fact.value,
                })
                .collect(),
            text_facts: value
                .text_facts
                .into_iter()
                .map(|fact| FfiBodyTextFact {
                    section: fact.section,
                    key: fact.key,
                    value: fact.value,
                })
                .collect(),
            materials: value
                .materials
                .into_iter()
                .map(|item| FfiBodyMaterial {
                    name: item.name,
                    percentage: item.percentage,
                })
                .collect(),
        }
    }
}
