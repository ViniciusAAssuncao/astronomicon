use super::celestial_body_details::CelestialBodyDetails;
use super::celestial_body_facts::push;
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::domain::{Star, StarKind};
use astronomicon_core::math::climate::local_radiative_equilibrium_temperature;
use astronomicon_core::math::radiometry::{orbital_irradiance, stellar_luminosity};
use astronomicon_core::units::{Duration, Irradiance, Position};
use std::collections::HashMap;
use uuid::Uuid;

fn star_flux_at_distance(star: &Star, distance_m: f64) -> Option<f64> {
    if !matches!(star.kind(), StarKind::Star | StarKind::BrownDwarf) {
        return None;
    }
    let radius = star.radius()?;
    let temperature = star.effective_temperature()?;
    if !radius.value().is_finite()
        || radius.value() <= 0.0
        || !temperature.value().is_finite()
        || temperature.value() <= 0.0
        || !distance_m.is_finite()
        || distance_m <= radius.value()
    {
        return None;
    }
    let luminosity = stellar_luminosity(radius, temperature);
    let irradiance = orbital_irradiance(
        luminosity,
        astronomicon_core::units::Length::new(distance_m),
    )
    .value();
    (irradiance.is_finite() && irradiance > 0.0).then_some(irradiance)
}

fn system_stellar_flux(
    stars: &[Star],
    positions: &HashMap<Uuid, Position>,
    body_id: Uuid,
) -> Option<f64> {
    let target = *positions.get(&body_id)?;
    if stars.is_empty() {
        return None;
    }
    let total = stars.iter().try_fold(0.0, |total, star| {
        let position = *positions.get(&star.id())?;
        let distance_m = (target - position).magnitude().value();
        let flux = star_flux_at_distance(star, distance_m)?;
        let combined = total + flux;
        combined.is_finite().then_some(combined)
    })?;
    (total > 0.0).then_some(total)
}

pub fn append_reference_environment(
    details: &mut CelestialBodyDetails,
    catalog: &EphemerisCatalog,
    id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) {
    let Some(body) = catalog.planets().iter().find(|body| body.id() == id) else {
        return;
    };
    let Some(system_id) = body.star_system_id() else {
        return;
    };
    if catalog.stars().is_empty() || !at_epoch.value().is_finite() {
        return;
    }
    let total_epoch = universe_epoch + at_epoch;
    if catalog.system_id != system_id || !total_epoch.value().is_finite() {
        return;
    }
    let Ok(positions) = catalog.positions(total_epoch) else {
        return;
    };
    let Some(incident_flux) = system_stellar_flux(catalog.stars(), &positions, id) else {
        return;
    };
    push(
        &mut details.facts,
        "environment",
        "stellar_irradiance",
        Some(incident_flux),
    );
    push(
        &mut details.facts,
        "environment",
        "diagnostic_epoch",
        Some(at_epoch.value()),
    );
    if let Some(albedo) = body
        .bond_albedo()
        .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
    {
        let equilibrium =
            local_radiative_equilibrium_temperature(Irradiance::new(incident_flux / 4.0), albedo);
        if equilibrium.value().is_finite() && equilibrium.value() > 0.0 {
            push(
                &mut details.facts,
                "environment",
                "radiative_equilibrium_k",
                Some(equilibrium.value()),
            );
        }
    }
    details.text_facts.push(super::celestial_body_facts::BodyTextFact {
        section: "environment".to_owned(),
        key: "reference_model".to_owned(),
        value: "Estimativa radiativa na época da seleção. Não representa temperatura superficial medida.".to_owned(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_nonphysical_stellar_flux() {
        let star = astronomicon_core::domain::Star::builder(
            Uuid::new_v4(),
            "Test",
            astronomicon_core::units::Mass::new(1.0e30),
            astronomicon_core::domain::StarKind::Star,
            astronomicon_core::domain::OrbitalParent::Fixed,
        )
        .with_radius(astronomicon_core::units::Length::new(6.0e8))
        .with_effective_temperature(astronomicon_core::units::Temperature::new(5000.0))
        .build()
        .unwrap();
        let nearby = star_flux_at_distance(&star, 1.0e11).unwrap();
        let distant = star_flux_at_distance(&star, 2.0e11).unwrap();
        assert!((distant / nearby - 0.25).abs() < 1.0e-12);
        assert!(star_flux_at_distance(&star, 6.0e8).is_none());
        assert!(star_flux_at_distance(&star, f64::NAN).is_none());
        assert!(star_flux_at_distance(&star, f64::INFINITY).is_none());
    }

    #[test]
    fn multi_star_flux_requires_every_source_and_tracks_geometry() {
        let make_star = |id| {
            Star::builder(
                id,
                "Radiative test star",
                astronomicon_core::units::Mass::new(1.0e30),
                StarKind::Star,
                astronomicon_core::domain::OrbitalParent::Fixed,
            )
            .with_radius(astronomicon_core::units::Length::new(6.0e8))
            .with_effective_temperature(astronomicon_core::units::Temperature::new(5000.0))
            .build()
            .unwrap()
        };
        let first = make_star(Uuid::new_v4());
        let second = make_star(Uuid::new_v4());
        let body = Uuid::new_v4();
        let mut positions = HashMap::from([
            (body, Position::from_components(0.0, 0.0, 0.0)),
            (first.id(), Position::from_components(1.0e11, 0.0, 0.0)),
            (second.id(), Position::from_components(0.0, 2.0e11, 0.0)),
        ]);
        let expected = star_flux_at_distance(&first, 1.0e11).unwrap()
            + star_flux_at_distance(&second, 2.0e11).unwrap();
        let actual =
            system_stellar_flux(&[first.clone(), second.clone()], &positions, body).unwrap();
        assert!((actual / expected - 1.0).abs() < 1.0e-12);
        positions.remove(&second.id());
        assert!(system_stellar_flux(&[first.clone(), second.clone()], &positions, body).is_none());
        positions.insert(second.id(), Position::from_components(0.0, 4.0e11, 0.0));
        let moved = system_stellar_flux(&[first, second], &positions, body).unwrap();
        assert!(moved < actual);
        assert!(moved > 0.0);
    }
}
