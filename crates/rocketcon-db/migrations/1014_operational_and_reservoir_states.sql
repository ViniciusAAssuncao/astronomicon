CREATE TABLE IF NOT EXISTS component_operational_states (
    vehicle_component_id TEXT PRIMARY KEY NOT NULL REFERENCES vehicle_components(id) ON DELETE CASCADE,
    load_fraction REAL NOT NULL CHECK (load_fraction >= 0.0 AND load_fraction <= 1.0),
    captured_universe_epoch_s REAL NOT NULL,
    captured_at_epoch_s REAL NOT NULL
);

CREATE TABLE IF NOT EXISTS energy_reservoir_states (
    vehicle_component_id TEXT PRIMARY KEY NOT NULL REFERENCES vehicle_components(id) ON DELETE CASCADE,
    stored_energy_j REAL NOT NULL CHECK (stored_energy_j >= 0.0),
    captured_universe_epoch_s REAL NOT NULL,
    captured_at_epoch_s REAL NOT NULL
);
