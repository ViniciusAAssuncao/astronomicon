use crate::error::DbResult;
use crate::models::MinorPlanetRow;
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn get_by_id(pool: &SqlitePool, id: &Uuid) -> DbResult<Option<MinorPlanetRow>> {
    let row = sqlx::query_as::<_, MinorPlanetRow>(
        "SELECT id, star_system_id, parent_star_id, parent_planet_id, parent_barycenter_id, parent_minor_planet_id, \
         name, spectral_type, mass_kg, axis_a_m, axis_b_m, axis_c_m, rotation_period_s, axial_tilt_rad, \
         macroporosity, geometric_albedo, bond_albedo, semi_major_axis_m, \
         eccentricity, inclination_rad, longitude_ascending_node_rad, argument_periapsis_rad, \
         mean_anomaly_at_epoch_rad \
         FROM minor_planets WHERE id = ?",
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn list_all(pool: &SqlitePool) -> DbResult<Vec<MinorPlanetRow>> {
    let rows = sqlx::query_as::<_, MinorPlanetRow>(
        "SELECT id, star_system_id, parent_star_id, parent_planet_id, parent_barycenter_id, parent_minor_planet_id, \
         name, spectral_type, mass_kg, axis_a_m, axis_b_m, axis_c_m, rotation_period_s, axial_tilt_rad, \
         macroporosity, geometric_albedo, bond_albedo, semi_major_axis_m, \
         eccentricity, inclination_rad, longitude_ascending_node_rad, argument_periapsis_rad, \
         mean_anomaly_at_epoch_rad \
         FROM minor_planets ORDER BY name ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn list_by_system(pool: &SqlitePool, system_id: &Uuid) -> DbResult<Vec<MinorPlanetRow>> {
    let rows = sqlx::query_as::<_, MinorPlanetRow>(
        "SELECT id, star_system_id, parent_star_id, parent_planet_id, parent_barycenter_id, parent_minor_planet_id, \
         name, spectral_type, mass_kg, axis_a_m, axis_b_m, axis_c_m, rotation_period_s, axial_tilt_rad, \
         macroporosity, geometric_albedo, bond_albedo, semi_major_axis_m, \
         eccentricity, inclination_rad, longitude_ascending_node_rad, argument_periapsis_rad, \
         mean_anomaly_at_epoch_rad \
         FROM minor_planets WHERE star_system_id = ? ORDER BY name ASC",
    )
    .bind(system_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn list_by_star(
    pool: &SqlitePool,
    parent_star_id: &Uuid,
) -> DbResult<Vec<MinorPlanetRow>> {
    let rows = sqlx::query_as::<_, MinorPlanetRow>(
        "SELECT id, star_system_id, parent_star_id, parent_planet_id, parent_barycenter_id, parent_minor_planet_id, \
         name, spectral_type, mass_kg, axis_a_m, axis_b_m, axis_c_m, rotation_period_s, axial_tilt_rad, \
         macroporosity, geometric_albedo, bond_albedo, semi_major_axis_m, \
         eccentricity, inclination_rad, longitude_ascending_node_rad, argument_periapsis_rad, \
         mean_anomaly_at_epoch_rad \
         FROM minor_planets WHERE parent_star_id = ? ORDER BY name ASC",
    )
    .bind(parent_star_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn list_by_planet(
    pool: &SqlitePool,
    parent_planet_id: &Uuid,
) -> DbResult<Vec<MinorPlanetRow>> {
    let rows = sqlx::query_as::<_, MinorPlanetRow>(
        "SELECT id, star_system_id, parent_star_id, parent_planet_id, parent_barycenter_id, parent_minor_planet_id, \
         name, spectral_type, mass_kg, axis_a_m, axis_b_m, axis_c_m, rotation_period_s, axial_tilt_rad, \
         macroporosity, geometric_albedo, bond_albedo, semi_major_axis_m, \
         eccentricity, inclination_rad, longitude_ascending_node_rad, argument_periapsis_rad, \
         mean_anomaly_at_epoch_rad \
         FROM minor_planets WHERE parent_planet_id = ? ORDER BY name ASC",
    )
    .bind(parent_planet_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rows)
}