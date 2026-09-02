CREATE TABLE IF NOT EXISTS vehicle_physical_states (
    vehicle_id TEXT PRIMARY KEY NOT NULL REFERENCES vehicles(id) ON DELETE CASCADE,
    position_x_m REAL NOT NULL,
    position_y_m REAL NOT NULL,
    position_z_m REAL NOT NULL,
    velocity_x_m_s REAL NOT NULL,
    velocity_y_m_s REAL NOT NULL,
    velocity_z_m_s REAL NOT NULL,
    orientation_q_w REAL NOT NULL,
    orientation_q_x REAL NOT NULL,
    orientation_q_y REAL NOT NULL,
    orientation_q_z REAL NOT NULL,
    angular_velocity_x_rad_s REAL NOT NULL,
    angular_velocity_y_rad_s REAL NOT NULL,
    angular_velocity_z_rad_s REAL NOT NULL,
    reference_body_id TEXT NOT NULL,
    captured_universe_epoch_s REAL NOT NULL,
    captured_at_epoch_s REAL NOT NULL
);

CREATE TABLE IF NOT EXISTS component_payload_states (
    vehicle_component_id TEXT PRIMARY KEY NOT NULL REFERENCES vehicle_components(id) ON DELETE CASCADE,
    is_deployed INTEGER NOT NULL CHECK (is_deployed IN (0, 1)),
    captured_universe_epoch_s REAL NOT NULL,
    captured_at_epoch_s REAL NOT NULL
);