CREATE TABLE IF NOT EXISTS component_heat_shield_states (
    vehicle_component_id TEXT PRIMARY KEY NOT NULL REFERENCES vehicle_components(id) ON DELETE CASCADE,
    remaining_thickness_m REAL NOT NULL CHECK (remaining_thickness_m >= 0.0),
    surface_temperature_k REAL NOT NULL CHECK (surface_temperature_k >= 0.0),
    captured_universe_epoch_s REAL NOT NULL,
    captured_at_epoch_s REAL NOT NULL
);