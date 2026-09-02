use crate::error::DbResult;
use crate::models::PlanetRow;
use crate::repositories::fetch::{fetch_all, fetch_all_by_param, fetch_optional_by_param};
use sqlx::SqlitePool;
use uuid::Uuid;

const BASE_QUERY: &str = "SELECT id, star_system_id, parent_star_id, parent_planet_id, \
    parent_barycenter_id, parent_minor_planet_id, name, kind, mass_kg, equatorial_radius_m, \
    polar_radius_m, rotation_period_s, axial_tilt_rad, geometric_albedo, bond_albedo, \
    thermal_inertia, solstice_true_anomaly_rad, semi_major_axis_m, eccentricity, \
    inclination_rad, longitude_ascending_node_rad, argument_periapsis_rad, \
    mean_anomaly_at_epoch_rad, oblateness_j2, core_mass_fraction, radioactive_heating_rate, \
    magnetic_field_locked, love_number_k2, tidal_dissipation_factor_q, \
    mantle_hydration_fraction, dust_availability_factor, surface_roughness_m, \
    dust_particle_radius_m, volcanic_ash_particle_radius_m FROM planets";

pub async fn get_by_id(pool: &SqlitePool, id: &Uuid) -> DbResult<Option<PlanetRow>> {
    let query = format!("{BASE_QUERY} WHERE id = ?");
    fetch_optional_by_param(pool, &query, id.to_string()).await
}

pub async fn list_all(pool: &SqlitePool) -> DbResult<Vec<PlanetRow>> {
    let query = format!("{BASE_QUERY} ORDER BY name ASC");
    fetch_all(pool, &query).await
}

pub async fn list_children_of_planet(
    pool: &SqlitePool,
    parent_planet_id: &Uuid,
) -> DbResult<Vec<PlanetRow>> {
    let query = format!("{BASE_QUERY} WHERE parent_planet_id = ? ORDER BY name ASC");
    fetch_all_by_param(pool, &query, parent_planet_id.to_string()).await
}

pub async fn list_by_star(pool: &SqlitePool, parent_star_id: &Uuid) -> DbResult<Vec<PlanetRow>> {
    let query = format!("{BASE_QUERY} WHERE parent_star_id = ? ORDER BY name ASC");
    fetch_all_by_param(pool, &query, parent_star_id.to_string()).await
}

pub async fn list_by_system(pool: &SqlitePool, system_id: &Uuid) -> DbResult<Vec<PlanetRow>> {
    let query = format!("{BASE_QUERY} WHERE star_system_id = ? ORDER BY name ASC");
    fetch_all_by_param(pool, &query, system_id.to_string()).await
}