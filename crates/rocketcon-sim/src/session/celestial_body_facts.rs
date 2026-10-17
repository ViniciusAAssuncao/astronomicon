use astronomicon_core::domain::{MinorPlanet, OrbitalElements, Planet, Star, StarKind};
use astronomicon_core::math::gravity::gravitational_parameter;
use astronomicon_core::math::radiation::peak_wavelength;
use astronomicon_core::math::radiometry::{escape_velocity, stellar_luminosity};
use astronomicon_core::units::Length;
use std::f64::consts::PI;

#[derive(Debug, Clone, PartialEq)]
pub struct BodyFact {
    pub section: String,
    pub key: String,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BodyTextFact {
    pub section: String,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BodyMaterial {
    pub name: String,
    pub percentage: f64,
}

pub fn push(facts: &mut Vec<BodyFact>, section: &str, key: &str, value: Option<f64>) {
    if let Some(value) = value.filter(|v| v.is_finite()) {
        facts.push(BodyFact {
            section: section.to_owned(),
            key: key.to_owned(),
            value,
        });
    }
}

fn text(facts: &mut Vec<BodyTextFact>, section: &str, key: &str, value: String) {
    facts.push(BodyTextFact {
        section: section.to_owned(),
        key: key.to_owned(),
        value,
    });
}

fn angle(value: Option<astronomicon_core::units::Angle>) -> Option<f64> {
    value.map(|v| v.value() * 180.0 / PI)
}

pub fn orbit(facts: &mut Vec<BodyFact>, elements: Option<OrbitalElements>) {
    let Some(orb) = elements else { return };
    let a = orb.semi_major_axis().value();
    let e = orb.eccentricity();
    push(facts, "orbit", "periapsis", Some(a * (1.0 - e)));
    push(facts, "orbit", "apoapsis", Some(a * (1.0 + e)));
    push(
        facts,
        "orbit",
        "ascending_node",
        angle(Some(orb.longitude_of_ascending_node())),
    );
    push(
        facts,
        "orbit",
        "periapsis_argument",
        angle(Some(orb.argument_of_periapsis())),
    );
    push(
        facts,
        "orbit",
        "mean_anomaly",
        angle(Some(orb.mean_anomaly_at_epoch())),
    );
}

fn spectral_temperature_band(temperature_k: f64) -> Option<&'static str> {
    if !temperature_k.is_finite() || temperature_k < 2400.0 {
        return None;
    }
    Some(match temperature_k {
        t if t >= 30000.0 => "O",
        t if t >= 10000.0 => "B",
        t if t >= 7500.0 => "A",
        t if t >= 6000.0 => "F",
        t if t >= 5200.0 => "G",
        t if t >= 3700.0 => "K",
        _ => "M",
    })
}

pub fn star(facts: &mut Vec<BodyFact>, text_facts: &mut Vec<BodyTextFact>, body: &Star) {
    text(
        text_facts,
        "star",
        "star_kind",
        format!("{:?}", body.kind()),
    );
    if body.kind() == StarKind::Star {
        if let Some(band) = body
            .effective_temperature()
            .and_then(|temperature| spectral_temperature_band(temperature.value()))
        {
            text(
                text_facts,
                "star",
                "spectral_temperature_band",
                band.to_owned(),
            );
        }
    }
    push(facts, "star", "axial_tilt", angle(body.obliquity()));
    push(facts, "star", "metallicity", body.metallicity());
    push(facts, "star", "oblateness_j2", body.oblateness_j2());
    if let (Some(radius), Some(temperature)) = (body.radius(), body.effective_temperature()) {
        if body.kind() == astronomicon_core::domain::StarKind::Star
            || body.kind() == astronomicon_core::domain::StarKind::BrownDwarf
        {
            push(
                facts,
                "star",
                "luminosity",
                Some(stellar_luminosity(radius, temperature).value()),
            );
            push(
                facts,
                "star",
                "peak_wavelength",
                Some(peak_wavelength(temperature).value()),
            );
        }
    }
    if let Some(radius) = body.radius() {
        push(
            facts,
            "general",
            "escape_speed",
            Some(escape_velocity(gravitational_parameter(body.mass()), radius).value()),
        );
    }
    orbit(facts, body.orbital_elements());
}

pub fn planet(facts: &mut Vec<BodyFact>, text_facts: &mut Vec<BodyTextFact>, body: &Planet) {
    text(
        text_facts,
        "general",
        "planet_kind",
        format!("{:?}", body.kind()),
    );
    push(
        facts,
        "general",
        "equatorial_radius",
        body.equatorial_radius().map(|v| v.value()),
    );
    push(
        facts,
        "general",
        "polar_radius",
        body.polar_radius().map(|v| v.value()),
    );
    if let (Some(equatorial), Some(polar)) = (body.equatorial_radius(), body.polar_radius()) {
        push(
            facts,
            "general",
            "flattening",
            Some((equatorial.value() - polar.value()) / equatorial.value()),
        );
    }
    if let Some(radius) = body.equatorial_radius().or(body.polar_radius()) {
        push(
            facts,
            "general",
            "escape_speed",
            Some(escape_velocity(gravitational_parameter(body.mass()), radius).value()),
        );
    }
    push(facts, "orbit", "axial_tilt", angle(body.obliquity()));
    push(
        facts,
        "orbit",
        "solstice_anomaly",
        angle(body.solstice_true_anomaly()),
    );
    push(
        facts,
        "interior",
        "core_fraction",
        body.core_mass_fraction(),
    );
    push(
        facts,
        "interior",
        "radioactive_heating",
        body.radioactive_heating_rate(),
    );
    push(
        facts,
        "interior",
        "mantle_hydration",
        body.mantle_hydration_fraction(),
    );
    push(facts, "interior", "love_number", body.love_number_k2());
    push(
        facts,
        "interior",
        "tidal_dissipation",
        body.tidal_dissipation_factor_q(),
    );
    push(
        facts,
        "interior",
        "magnetic_locked",
        body.magnetic_field_locked().map(|v| v.value()),
    );
    push(
        facts,
        "surface",
        "geometric_albedo",
        body.geometric_albedo(),
    );
    push(facts, "surface", "bond_albedo", body.bond_albedo());
    push(facts, "surface", "thermal_inertia", body.thermal_inertia());
    push(
        facts,
        "surface",
        "surface_roughness",
        body.surface_roughness().map(|v| v.value()),
    );
    push(
        facts,
        "surface",
        "dust_availability",
        body.dust_availability_factor(),
    );
    push(
        facts,
        "surface",
        "dust_particle_radius",
        body.dust_particle_radius().map(|v| v.value()),
    );
    push(
        facts,
        "surface",
        "ash_particle_radius",
        body.volcanic_ash_particle_radius().map(|v| v.value()),
    );
    push(facts, "orbit", "oblateness_j2", body.oblateness_j2());
    orbit(facts, body.orbital_elements());
}

pub fn minor(facts: &mut Vec<BodyFact>, text_facts: &mut Vec<BodyTextFact>, body: &MinorPlanet) {
    text(
        text_facts,
        "general",
        "asteroid_spectral_type",
        format!("{:?}", body.spectral_type()),
    );
    push(facts, "general", "axis_a", body.axis_a().map(|v| v.value()));
    push(facts, "general", "axis_b", body.axis_b().map(|v| v.value()));
    push(facts, "general", "axis_c", body.axis_c().map(|v| v.value()));
    push(facts, "surface", "macroporosity", body.macroporosity());
    push(
        facts,
        "surface",
        "geometric_albedo",
        body.geometric_albedo(),
    );
    push(facts, "surface", "bond_albedo", body.bond_albedo());
    push(facts, "orbit", "axial_tilt", angle(body.obliquity()));
    let radii = [body.axis_a(), body.axis_b(), body.axis_c()];
    if let [Some(a), Some(b), Some(c)] = radii {
        let effective = (a.value() * b.value() * c.value()).cbrt();
        push(
            facts,
            "general",
            "escape_speed",
            Some(
                escape_velocity(gravitational_parameter(body.mass()), Length::new(effective))
                    .value(),
            ),
        );
    }
    orbit(facts, body.orbital_elements());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thermal_band_requires_appropriate_temperature_and_never_claims_a_subclass() {
        assert_eq!(spectral_temperature_band(5772.0), Some("G"));
        assert_eq!(spectral_temperature_band(6500.0), Some("F"));
        assert_eq!(spectral_temperature_band(4000.0), Some("K"));
        assert_eq!(spectral_temperature_band(31000.0), Some("O"));
        assert_eq!(spectral_temperature_band(2400.0), Some("M"));
        assert_eq!(spectral_temperature_band(2300.0), None);
        assert_eq!(spectral_temperature_band(f64::NAN), None);
    }
}
