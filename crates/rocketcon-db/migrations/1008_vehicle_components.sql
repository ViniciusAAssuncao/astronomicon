CREATE TABLE IF NOT EXISTS vehicle_components (
    id TEXT PRIMARY KEY NOT NULL,
    vehicle_id TEXT NOT NULL REFERENCES vehicles(id) ON DELETE CASCADE,
    component_id TEXT NOT NULL REFERENCES components(id) ON DELETE RESTRICT,
    instance_label TEXT,
    UNIQUE (vehicle_id, instance_label)
);