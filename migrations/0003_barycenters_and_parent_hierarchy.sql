CREATE TABLE IF NOT EXISTS barycenters (
    id TEXT PRIMARY KEY NOT NULL,
    star_system_id TEXT REFERENCES star_systems(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    primary_star_id TEXT REFERENCES stars(id) ON DELETE RESTRICT,
    primary_planet_id TEXT REFERENCES planets(id) ON DELETE RESTRICT,
    primary_barycenter_id TEXT REFERENCES barycenters(id) ON DELETE RESTRICT,
    secondary_star_id TEXT REFERENCES stars(id) ON DELETE RESTRICT,
    secondary_planet_id TEXT REFERENCES planets(id) ON DELETE RESTRICT,
    secondary_barycenter_id TEXT REFERENCES barycenters(id) ON DELETE RESTRICT,
    internal_semi_major_axis_m REAL NOT NULL,
    internal_eccentricity REAL NOT NULL,
    internal_inclination_rad REAL NOT NULL,
    internal_longitude_ascending_node_rad REAL NOT NULL,
    internal_argument_periapsis_rad REAL NOT NULL,
    internal_mean_anomaly_at_epoch_rad REAL NOT NULL,
    parent_star_id TEXT REFERENCES stars(id) ON DELETE RESTRICT,
    parent_planet_id TEXT REFERENCES planets(id) ON DELETE RESTRICT,
    parent_barycenter_id TEXT REFERENCES barycenters(id) ON DELETE RESTRICT,
    external_semi_major_axis_m REAL,
    external_eccentricity REAL,
    external_inclination_rad REAL,
    external_longitude_ascending_node_rad REAL,
    external_argument_periapsis_rad REAL,
    external_mean_anomaly_at_epoch_rad REAL
);

CREATE TABLE stars_new (
    id TEXT PRIMARY KEY NOT NULL,
    star_system_id TEXT REFERENCES star_systems(id) ON DELETE CASCADE,
    parent_star_id TEXT REFERENCES stars_new(id) ON DELETE RESTRICT,
    parent_planet_id TEXT REFERENCES planets(id) ON DELETE RESTRICT,
    parent_barycenter_id TEXT REFERENCES barycenters(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    mass_kg REAL NOT NULL,
    radius_m REAL,
    effective_temperature_k REAL,
    rotation_period_s REAL,
    axial_tilt_rad REAL,
    semi_major_axis_m REAL,
    eccentricity REAL,
    inclination_rad REAL,
    longitude_ascending_node_rad REAL,
    argument_periapsis_rad REAL,
    mean_anomaly_at_epoch_rad REAL
);

INSERT INTO stars_new (
    id, star_system_id, name, kind, mass_kg, radius_m,
    effective_temperature_k, rotation_period_s, axial_tilt_rad,
    semi_major_axis_m, eccentricity, inclination_rad,
    longitude_ascending_node_rad, argument_periapsis_rad, mean_anomaly_at_epoch_rad
)
SELECT
    id, star_system_id, name, kind, mass_kg, radius_m,
    effective_temperature_k, rotation_period_s, axial_tilt_rad,
    semi_major_axis_m, eccentricity, inclination_rad,
    longitude_ascending_node_rad, argument_periapsis_rad, mean_anomaly_at_epoch_rad
FROM stars;

DROP TABLE stars;
ALTER TABLE stars_new RENAME TO stars;

CREATE TABLE planets_new (
    id TEXT PRIMARY KEY NOT NULL,
    star_system_id TEXT REFERENCES star_systems(id) ON DELETE CASCADE,
    parent_star_id TEXT REFERENCES stars(id) ON DELETE RESTRICT,
    parent_planet_id TEXT REFERENCES planets_new(id) ON DELETE RESTRICT,
    parent_barycenter_id TEXT REFERENCES barycenters(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    mass_kg REAL NOT NULL,
    equatorial_radius_m REAL,
    polar_radius_m REAL,
    rotation_period_s REAL,
    axial_tilt_rad REAL,
    geometric_albedo REAL,
    bond_albedo REAL,
    thermal_inertia REAL,
    solstice_true_anomaly_rad REAL,
    semi_major_axis_m REAL,
    eccentricity REAL,
    inclination_rad REAL,
    longitude_ascending_node_rad REAL,
    argument_periapsis_rad REAL,
    mean_anomaly_at_epoch_rad REAL
);

INSERT INTO planets_new (
    id, parent_star_id, parent_planet_id, name, kind, mass_kg,
    equatorial_radius_m, polar_radius_m, rotation_period_s,
    axial_tilt_rad, geometric_albedo, bond_albedo, thermal_inertia,
    solstice_true_anomaly_rad, semi_major_axis_m, eccentricity,
    inclination_rad, longitude_ascending_node_rad,
    argument_periapsis_rad, mean_anomaly_at_epoch_rad
)
SELECT
    id, parent_star_id, parent_planet_id, name, kind, mass_kg,
    equatorial_radius_m, polar_radius_m, rotation_period_s,
    axial_tilt_rad, geometric_albedo, bond_albedo, thermal_inertia,
    solstice_true_anomaly_rad, semi_major_axis_m, eccentricity,
    inclination_rad, longitude_ascending_node_rad,
    argument_periapsis_rad, mean_anomaly_at_epoch_rad
FROM planets;

DROP TABLE planets;
ALTER TABLE planets_new RENAME TO planets;