ALTER TABLE component_operational_states ADD COLUMN current_gimbal_pitch_rad REAL;
ALTER TABLE component_operational_states ADD COLUMN current_gimbal_yaw_rad REAL;

CREATE TABLE IF NOT EXISTS component_reaction_wheel_states (
    vehicle_component_id TEXT PRIMARY KEY NOT NULL REFERENCES vehicle_components(id) ON DELETE CASCADE,
    stored_angular_momentum_n_m_s REAL NOT NULL,
    captured_universe_epoch_s REAL NOT NULL,
    captured_at_epoch_s REAL NOT NULL
);
