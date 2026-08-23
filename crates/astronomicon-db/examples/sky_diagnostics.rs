use astronomicon_app::resolve_sky_diagnostics;
use astronomicon_core::units::Duration;
use astronomicon_db::repositories::{atmosphere_repository, planet_repository, universe_state_repository};
use astronomicon_db::save::initialize_save;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = initialize_save("sqlite://database/astronomicon.db").await?;

    let state_row = universe_state_repository::get(&pool).await?.unwrap();
    let universe_epoch = Duration::new(state_row.seconds_since_j2000_epoch);
    let at_epoch = Duration::new(0.0);

    let planets = planet_repository::list_all(&pool).await?;

    for planet_row in planets {
        let planet_id = Uuid::parse_str(&planet_row.id)?;
        let atmosphere = atmosphere_repository::get_by_planet_id(&pool, &planet_id).await?;

        if atmosphere.is_some() {
            if let Some(sky) = resolve_sky_diagnostics(&pool, planet_id, universe_epoch, at_epoch).await? {
                println!("Planeta: {}", planet_row.name);
                println!("{}", serde_json::to_string_pretty(&sky)?);
                println!("--------------------------------------------------");
            }
        }
    }

    Ok(())
}