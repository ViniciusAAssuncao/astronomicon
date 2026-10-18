use super::celestial_body_details::CelestialBodyDetails;
use super::celestial_body_environment::RadiativeContext;
use super::celestial_body_facts::push;
use astronomicon_app::climate::resolve_stellar_wind_at_distance;
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_app::geophysics::resolve_magnetic_field;
use astronomicon_core::math::magnetic_field::magnetopause_radius;
use astronomicon_core::math::radiation::stellar_particle_flux;
use astronomicon_core::units::{Duration, Pressure};
use astronomicon_db::SqlitePool;
use uuid::Uuid;

pub async fn append_radiation_and_magnetism(
    details: &mut CelestialBodyDetails,
    pool: &SqlitePool,
    catalog: &EphemerisCatalog,
    id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
    context: &RadiativeContext,
) {
    let Some(body) = catalog.planets().iter().find(|planet| planet.id() == id) else {
        return;
    };
    let Some(target) = context.positions.get(&id).copied() else {
        return;
    };
    let mut pressure_pa = 0.0;
    let mut particles_w_m2 = 0.0;
    for star in catalog.stars() {
        let Some(source) = context.positions.get(&star.id()).copied() else {
            return;
        };
        let Some(radius) = star.radius() else {
            return;
        };
        let Some(temperature) = star.effective_temperature() else {
            return;
        };
        let distance = (target - source).magnitude();
        if !distance.value().is_finite() || distance.value() <= radius.value() {
            return;
        }
        let wind =
            resolve_stellar_wind_at_distance(star.mass(), radius, temperature, distance, 1.0, 1.0);
        let pressure = wind.dynamic_pressure.value();
        let energy =
            stellar_particle_flux(wind.wind_density_at_orbit, wind.terminal_wind_speed).value();
        if !pressure.is_finite() || pressure < 0.0 || !energy.is_finite() || energy < 0.0 {
            return;
        }
        pressure_pa += pressure;
        particles_w_m2 += energy;
    }
    if !pressure_pa.is_finite() || !particles_w_m2.is_finite() {
        return;
    }
    if pressure_pa > 0.0 {
        push(
            &mut details.facts,
            "radiation",
            "wind_pressure_pa",
            Some(pressure_pa),
        );
    }
    if particles_w_m2 > 0.0 {
        push(
            &mut details.facts,
            "radiation",
            "stellar_particle_flux",
            Some(particles_w_m2),
        );
    }

    let has_field = body.magnetic_field_locked().is_some()
        || (body.core_mass_fraction().is_some_and(|cmf| cmf > 0.0)
            && body.rotation_period().is_some());
    if !has_field || body.equatorial_radius().is_none() {
        return;
    }
    let Ok(field) = resolve_magnetic_field(pool, id, 1.0, 1.0, universe_epoch, at_epoch).await
    else {
        return;
    };
    let equatorial = field.equatorial_magnetic_field.value();
    let polar = field.polar_magnetic_field.value();
    if !equatorial.is_finite() || equatorial < 0.0 || !polar.is_finite() || polar < 0.0 {
        return;
    }
    push(
        &mut details.facts,
        "magnetic",
        "magnetic_equatorial_t",
        Some(equatorial),
    );
    push(
        &mut details.facts,
        "magnetic",
        "magnetic_polar_t",
        Some(polar),
    );
    if pressure_pa > 0.0 {
        let boundary = magnetopause_radius(field.dipole_moment, Pressure::new(pressure_pa)).value();
        let radius = body.equatorial_radius().map(|r| r.value()).unwrap_or(0.0);
        if boundary.is_finite() && boundary > radius && radius > 0.0 {
            push(
                &mut details.facts,
                "magnetic",
                "magnetopause_m",
                Some(boundary),
            );
            push(
                &mut details.facts,
                "magnetic",
                "magnetopause_radii",
                Some(boundary / radius),
            );
        }
    }
}
