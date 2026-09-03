CREATE TABLE IF NOT EXISTS vehicle_trajectory_patches_new (
    id TEXT PRIMARY KEY NOT NULL,
    vehicle_id TEXT NOT NULL REFERENCES vehicles(id) ON DELETE CASCADE,
    reference_body_id TEXT NOT NULL,
    start_universe_epoch_s REAL NOT NULL,
    end_universe_epoch_s REAL,
    gravitational_parameter_m3_s2 REAL NOT NULL CHECK (gravitational_parameter_m3_s2 > 0),
    patch_type TEXT NOT NULL DEFAULT 'conic' CHECK (patch_type IN ('conic', 'low_thrust')),
    semi_major_axis_m REAL,
    eccentricity REAL CHECK (eccentricity IS NULL OR eccentricity >= 0),
    inclination_rad REAL,
    longitude_of_ascending_node_rad REAL,
    argument_of_periapsis_rad REAL,
    true_anomaly_at_epoch_rad REAL,
    initial_mass_kg REAL CHECK (initial_mass_kg IS NULL OR initial_mass_kg > 0),
    final_mass_kg REAL CHECK (final_mass_kg IS NULL OR final_mass_kg > 0),
    thrust_n REAL CHECK (thrust_n IS NULL OR thrust_n > 0),
    specific_impulse_s REAL CHECK (specific_impulse_s IS NULL OR specific_impulse_s > 0),
    total_delta_v_m_s REAL CHECK (total_delta_v_m_s IS NULL OR total_delta_v_m_s >= 0),
    chebyshev_x_json TEXT,
    chebyshev_y_json TEXT,
    chebyshev_z_json TEXT,
    chebyshev_vx_json TEXT,
    chebyshev_vy_json TEXT,
    chebyshev_vz_json TEXT,
    chebyshev_mass_json TEXT
);

INSERT INTO vehicle_trajectory_patches_new (
    id, vehicle_id, reference_body_id,
    start_universe_epoch_s, end_universe_epoch_s,
    gravitational_parameter_m3_s2, patch_type,
    semi_major_axis_m, eccentricity, inclination_rad,
    longitude_of_ascending_node_rad, argument_of_periapsis_rad,
    true_anomaly_at_epoch_rad
)
SELECT
    id, vehicle_id, reference_body_id,
    start_universe_epoch_s, end_universe_epoch_s,
    gravitational_parameter_m3_s2, 'conic',
    semi_major_axis_m, eccentricity, inclination_rad,
    longitude_of_ascending_node_rad, argument_of_periapsis_rad,
    true_anomaly_at_epoch_rad
FROM vehicle_trajectory_patches;

DROP TABLE vehicle_trajectory_patches;

ALTER TABLE vehicle_trajectory_patches_new RENAME TO vehicle_trajectory_patches;

CREATE INDEX IF NOT EXISTS idx_vehicle_trajectory_patches_lookup
ON vehicle_trajectory_patches (vehicle_id, start_universe_epoch_s);

CREATE INDEX IF NOT EXISTS idx_vehicle_trajectory_patches_type
ON vehicle_trajectory_patches (vehicle_id, patch_type, start_universe_epoch_s);