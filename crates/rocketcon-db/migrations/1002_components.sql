CREATE TABLE IF NOT EXISTS components (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    component_kind TEXT NOT NULL CHECK (component_kind IN ('Engine', 'PropellantTank', 'Battery', 'SolarPanel', 'Cpu')),
    dry_mass_kg REAL NOT NULL CHECK (dry_mass_kg > 0),
    length_m REAL NOT NULL CHECK (length_m > 0),
    diameter_m REAL NOT NULL CHECK (diameter_m > 0),
    power_consumption_w REAL NOT NULL DEFAULT 0.0 CHECK (power_consumption_w >= 0),
    manufacturer TEXT,
    manufactured_at_unix_seconds INTEGER,
    lore_notes TEXT
);