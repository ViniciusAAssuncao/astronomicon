CREATE TABLE IF NOT EXISTS vehicle_trajectory_patches (
    id TEXT PRIMARY KEY NOT NULL,
    vehicle_id TEXT NOT NULL REFERENCES vehicles(id) ON DELETE CASCADE,
    reference_body_id TEXT NOT NULL,
    start_universe_epoch_s REAL NOT NULL,
    end_universe_epoch_s REAL,
    semi_major_axis_m REAL NOT NULL,
    eccentricity REAL NOT NULL CHECK (eccentricity >= 0),
    inclination_rad REAL NOT NULL,
    longitude_of_ascending_node_rad REAL NOT NULL,
    argument_of_periapsis_rad REAL NOT NULL,
    true_anomaly_at_epoch_rad REAL NOT NULL,
    gravitational_parameter_m3_s2 REAL NOT NULL CHECK (gravitational_parameter_m3_s2 > 0)
);

CREATE INDEX IF NOT EXISTS idx_vehicle_trajectory_patches_lookup
ON vehicle_trajectory_patches (vehicle_id, start_universe_epoch_s);