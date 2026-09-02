CREATE TABLE IF NOT EXISTS vehicles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    vehicle_kind TEXT NOT NULL CHECK (vehicle_kind IN ('Rocket', 'Spacecraft', 'Probe', 'Rover', 'Satellite')),
    manufacturer TEXT,
    manufactured_at_unix_seconds INTEGER,
    lore_notes TEXT
);