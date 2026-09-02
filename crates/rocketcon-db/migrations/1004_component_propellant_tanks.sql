CREATE TABLE IF NOT EXISTS component_propellant_tanks (
    component_id TEXT PRIMARY KEY NOT NULL REFERENCES components(id) ON DELETE CASCADE,
    propellant_id TEXT NOT NULL REFERENCES propellants(id) ON DELETE RESTRICT,
    max_propellant_mass_kg REAL NOT NULL CHECK (max_propellant_mass_kg > 0)
);