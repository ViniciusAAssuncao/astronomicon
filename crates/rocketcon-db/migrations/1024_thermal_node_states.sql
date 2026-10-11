CREATE TABLE IF NOT EXISTS thermal_node_states (
    vehicle_component_id TEXT PRIMARY KEY NOT NULL REFERENCES vehicle_components(id) ON DELETE CASCADE,
    current_temperature_k REAL NOT NULL CHECK (current_temperature_k >= 0.0),
    captured_universe_epoch_s REAL NOT NULL,
    captured_at_epoch_s REAL NOT NULL
);