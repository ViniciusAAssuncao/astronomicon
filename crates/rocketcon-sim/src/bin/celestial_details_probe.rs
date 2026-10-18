use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::units::Duration;
use astronomicon_db::connection::open_read_only_pool_path;
use astronomicon_db::repositories::system_repository;
use rocketcon_sim::{append_reference_environment, selected_body_details};
use std::error::Error;
use std::path::Path;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let save = std::env::args().nth(1).ok_or("save path required")?;
    let pool = open_read_only_pool_path(Path::new(&save)).await?;
    let systems = system_repository::list_all(&pool).await?;
    let universe_epoch = rocketcon_app::universe::resolve_universe_epoch(&pool).await?;
    let mut radiative = 0usize;
    let mut equilibria = 0usize;
    let mut stars = 0usize;
    let mut planets = 0usize;
    let mut atmospheres = 0usize;
    let mut hydrospheres = 0usize;
    let mut geological = 0usize;
    let mut stars_luminous = 0usize;
    let mut orbital = 0usize;
    for system in &systems {
        let id = Uuid::parse_str(&system.id)?;
        let catalog = EphemerisCatalog::load(&pool, id).await?;
        for star in catalog.stars() {
            let details = selected_body_details(&pool, &catalog, star.id()).await?;
            if !details
                .text_facts
                .iter()
                .any(|fact| fact.key == "star_kind")
            {
                return Err("missing stellar kind".into());
            }
            if details.facts.iter().any(|fact| fact.key == "luminosity") {
                stars_luminous += 1;
            }
            let band_present = details
                .text_facts
                .iter()
                .any(|fact| fact.key == "spectral_temperature_band");
            let band_eligible = star.kind() == astronomicon_core::domain::StarKind::Star
                && star.effective_temperature().is_some_and(|temperature| {
                    temperature.value().is_finite() && temperature.value() >= 2400.0
                });
            if band_present != band_eligible {
                return Err("incorrect temperature-derived stellar band eligibility".into());
            }
            if details.atmosphere.is_some() || details.hydrosphere.is_some() {
                return Err("star has inappropriate planetary environment".into());
            }
            stars += 1;
        }
        for body in catalog.planets() {
            let mut details = selected_body_details(&pool, &catalog, body.id()).await?;
            append_reference_environment(
                &mut details,
                &catalog,
                body.id(),
                universe_epoch,
                Duration::new(0.0),
            );
            if details
                .facts
                .iter()
                .any(|fact| fact.key == "stellar_irradiance")
            {
                radiative += 1;

            }
            if details
                .facts
                .iter()
                .any(|fact| fact.key == "radiative_equilibrium_k")
            {
                equilibria += 1;
            }
            if details.id != body.id()
                || !details
                    .text_facts
                    .iter()
                    .any(|fact| fact.key == "planet_kind")
            {
                return Err("incomplete planet details".into());
            }
            if details.facts.iter().any(|fact| fact.key == "periapsis") {
                orbital += 1;
            }
            if details.atmosphere.is_some() {
                atmospheres += 1;
            }
            if details.hydrosphere.is_some() {
                hydrospheres += 1;
            }
            if !details.materials.is_empty() {
                geological += 1;
            }
            planets += 1;
        }
        for body in catalog.minor_planets() {
            let details = selected_body_details(&pool, &catalog, body.id()).await?;
            if !details
                .text_facts
                .iter()
                .any(|fact| fact.key == "asteroid_spectral_type")
            {
                return Err("missing minor body type".into());
            }
        }
    }
    if radiative < 3 || equilibria < 3 {
        return Err(format!(
            "reference environment incomplete: flux={} equilibrium={}",
            radiative, equilibria
        )
        .into());
    }
    if stars < 4
        || planets < 10
        || atmospheres < 3
        || hydrospheres < 3
        || geological < 4
        || stars_luminous < 2
        || orbital < 4
    {
        return Err(format!("insufficient static catalog coverage: stars={} planets={} atmos={} hydro={} geology={} luminous={} orbital={}",
            stars, planets, atmospheres, hydrospheres, geological, stars_luminous, orbital).into());
    }
    println!(
        "CELESTIAL_DETAILS_PASS systems={} stars={} luminous={} planets={} atmospheres={} hydrospheres={} geology={} orbital={} radiative={} equilibria={}",
        systems.len(),
        stars,
        stars_luminous,
        planets,
        atmospheres,
        hydrospheres,
        geological,
        orbital,
        radiative,
        equilibria
    );
    pool.close().await;
    Ok(())
}
