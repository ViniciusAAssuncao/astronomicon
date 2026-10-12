CREATE TABLE IF NOT EXISTS engine_ignition_history (
    vehicle_component_id TEXT PRIMARY KEY NOT NULL REFERENCES vehicle_components(id) ON DELETE CASCADE,
    ignition_count INTEGER NOT NULL CHECK (ignition_count >= 0)
);
