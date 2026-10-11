use astronomicon_app::climate::temperature::ClimateBodyInputs;
use astronomicon_app::hierarchy::{find_parent_star, find_parent_star_on_connection};
use astronomicon_core::domain::OrbitalParent;
use astronomicon_db::connection::open_pool;
use astronomicon_db::repositories::{atmosphere_repository, planet_repository};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, SqliteConnection};
use std::error::Error;
use std::path::Path;
use uuid::Uuid;

#[tokio::test]
async fn climate_body_reads_one_committed_snapshot() -> Result<(), Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!("rocketcon-climate-{}.db", Uuid::new_v4()));
    let template = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../database/astronomicon.db");
    let options = SqliteConnectOptions::new()
        .filename(template)
        .read_only(true);
    let mut source = SqliteConnection::connect_with(&options).await?;
    sqlx::query("VACUUM INTO ?")
        .bind(path.to_str().ok_or("invalid temporary database path")?)
        .execute(&mut source)
        .await?;
    source.close().await?;
    let url = format!("sqlite://{}", path.display());
    let pool = open_pool(&url).await?;
    let planet_id = Uuid::parse_str(super::HADAB_ID)?;
    let before = ClimateBodyInputs::load(&pool, planet_id).await?;

    let mut reader = pool.begin().await?;
    let planet_before = planet_repository::get_by_id_on_connection(&mut reader, &planet_id)
        .await?
        .ok_or("planet missing")?;
    let atmosphere_before = atmosphere_repository::get_by_planet_id(&pool, &planet_id)
        .await?
        .ok_or("atmosphere missing")?;

    let mut writer = pool.begin().await?;
    sqlx::query("UPDATE planets SET bond_albedo = bond_albedo + 0.01 WHERE id = ?")
        .bind(planet_id.to_string())
        .execute(&mut *writer)
        .await?;
    sqlx::query(
        "UPDATE atmospheres SET greenhouse_effect_k = greenhouse_effect_k + 1 WHERE planet_id = ?",
    )
    .bind(planet_id.to_string())
    .execute(&mut *writer)
    .await?;
    writer.commit().await?;

    let planet_during = planet_repository::get_by_id_on_connection(&mut reader, &planet_id)
        .await?
        .ok_or("planet missing during snapshot")?;
    let atmosphere_during =
        atmosphere_repository::get_by_planet_id_on_connection(&mut reader, &planet_id)
            .await?
            .ok_or("atmosphere missing during snapshot")?;
    assert_eq!(planet_before.bond_albedo, planet_during.bond_albedo);
    assert_eq!(atmosphere_before, atmosphere_during);
    reader.rollback().await?;

    let planet_after = planet_repository::get_by_id(&pool, &planet_id)
        .await?
        .ok_or("planet missing after commit")?;
    let atmosphere_after = atmosphere_repository::get_by_planet_id(&pool, &planet_id)
        .await?
        .ok_or("atmosphere missing after commit")?;
    assert_ne!(planet_before.bond_albedo, planet_after.bond_albedo);
    assert_ne!(atmosphere_before, atmosphere_after);
    assert_ne!(before, ClimateBodyInputs::load(&pool, planet_id).await?);

    let stars = sqlx::query_as::<_, (String, f64)>(
        "SELECT id, mass_kg FROM stars ORDER BY mass_kg DESC LIMIT 2",
    )
    .fetch_all(&pool)
    .await?;
    assert_eq!(stars.len(), 2);
    let barycenter_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO barycenters (id, star_system_id, name, primary_star_id, secondary_star_id, \
         internal_semi_major_axis_m, internal_eccentricity, internal_inclination_rad, \
         internal_longitude_ascending_node_rad, internal_argument_periapsis_rad, \
         internal_mean_anomaly_at_epoch_rad) \
         VALUES (?, (SELECT star_system_id FROM stars WHERE id = ?), ?, ?, ?, 1, 0, 0, 0, 0, 0)",
    )
    .bind(barycenter_id.to_string())
    .bind(&stars[0].0)
    .bind("Climate snapshot test")
    .bind(&stars[0].0)
    .bind(&stars[1].0)
    .execute(&pool)
    .await?;
    let parent = OrbitalParent::Barycenter(barycenter_id);
    let expected = find_parent_star(&pool, parent).await?;
    let mut hierarchy_reader = pool.begin().await?;
    let actual = find_parent_star_on_connection(&mut hierarchy_reader, parent).await?;
    hierarchy_reader.rollback().await?;
    assert_eq!(expected.id(), Uuid::parse_str(&stars[0].0)?);
    assert_eq!(expected, actual);

    pool.close().await;
    drop(pool);
    for attempt in 0..10 {
        match std::fs::remove_file(&path) {
            Ok(()) => break,
            Err(error) if attempt < 9 => {
                let _ = error;
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}
