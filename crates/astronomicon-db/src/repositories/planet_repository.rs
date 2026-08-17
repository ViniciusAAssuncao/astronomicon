use crate::error::DbResult;
use crate::models::PlanetRow;
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn get_by_id(pool: &SqlitePool, id: &Uuid) -> DbResult<Option<PlanetRow>> {
    let row = sqlx::query_as::<_, PlanetRow>(
        "SELECT id, parent_star_id, parent_planet_id, name, kind, mass_kg, equatorial_radius_m, \
         polar_radius_m, rotation_period_s, axial_tilt_rad, geometric_albedo, bond_albedo, \
         thermal_inertia, solstice_true_anomaly_rad, semi_major_axis_m, eccentricity, inclination_rad, \
         longitude_ascending_node_rad, argument_periapsis_rad, mean_anomaly_at_epoch_rad \
         FROM planets WHERE id = ?",
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn list_all(pool: &SqlitePool) -> DbResult<Vec<PlanetRow>> {
    let rows = sqlx::query_as::<_, PlanetRow>(
        "SELECT id, parent_star_id, parent_planet_id, name, kind, mass_kg, equatorial_radius_m, \
         polar_radius_m, rotation_period_s, axial_tilt_rad, geometric_albedo, bond_albedo, \
         thermal_inertia, solstice_true_anomaly_rad, semi_major_axis_m, eccentricity, inclination_rad, \
         longitude_ascending_node_rad, argument_periapsis_rad, mean_anomaly_at_epoch_rad \
         FROM planets ORDER BY name ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn list_children_of_planet(
    pool: &SqlitePool,
    parent_planet_id: &Uuid,
) -> DbResult<Vec<PlanetRow>> {
    let rows = sqlx::query_as::<_, PlanetRow>(
        "SELECT id, parent_star_id, parent_planet_id, name, kind, mass_kg, equatorial_radius_m, \
         polar_radius_m, rotation_period_s, axial_tilt_rad, geometric_albedo, bond_albedo, \
         thermal_inertia, solstice_true_anomaly_rad, semi_major_axis_m, eccentricity, inclination_rad, \
         longitude_ascending_node_rad, argument_periapsis_rad, mean_anomaly_at_epoch_rad \
         FROM planets WHERE parent_planet_id = ? ORDER BY name ASC",
    )
    .bind(parent_planet_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn list_by_star(pool: &SqlitePool, parent_star_id: &Uuid) -> DbResult<Vec<PlanetRow>> {
    let rows = sqlx::query_as::<_, PlanetRow>(
        "SELECT id, parent_star_id, parent_planet_id, name, kind, mass_kg, equatorial_radius_m, \
         polar_radius_m, rotation_period_s, axial_tilt_rad, geometric_albedo, bond_albedo, \
         thermal_inertia, solstice_true_anomaly_rad, semi_major_axis_m, eccentricity, inclination_rad, \
         longitude_ascending_node_rad, argument_periapsis_rad, mean_anomaly_at_epoch_rad \
         FROM planets WHERE parent_star_id = ? ORDER BY name ASC",
    )
    .bind(parent_star_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn list_by_system(pool: &SqlitePool, system_id: &Uuid) -> DbResult<Vec<PlanetRow>> {
    let rows = sqlx::query_as::<_, PlanetRow>(
        "WITH RECURSIVE system_stars AS (
            SELECT id FROM stars WHERE star_system_id = ?
        ),
        system_planets AS (
            SELECT * FROM planets WHERE parent_star_id IN (SELECT id FROM system_stars)
            UNION ALL
            SELECT p.* FROM planets p JOIN system_planets sp ON p.parent_planet_id = sp.id
        )
        SELECT * FROM system_planets ORDER BY name ASC",
    )
    .bind(system_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
