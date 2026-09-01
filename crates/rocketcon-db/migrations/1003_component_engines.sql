CREATE TABLE IF NOT EXISTS component_engines (
    component_id TEXT PRIMARY KEY NOT NULL REFERENCES components(id) ON DELETE CASCADE,
    fuel_propellant_id TEXT NOT NULL REFERENCES propellants(id) ON DELETE RESTRICT,
    oxidizer_propellant_id TEXT REFERENCES propellants(id) ON DELETE RESTRICT,
    specific_impulse_vacuum_s REAL NOT NULL CHECK (specific_impulse_vacuum_s > 0),
    specific_impulse_sea_level_s REAL CHECK (specific_impulse_sea_level_s IS NULL OR specific_impulse_sea_level_s > 0),
    max_thrust_n REAL NOT NULL CHECK (max_thrust_n > 0),
    ignition_type TEXT NOT NULL CHECK (ignition_type IN ('Restartable', 'SingleBurn')),
    integral_propellant_mass_kg REAL CHECK (integral_propellant_mass_kg IS NULL OR integral_propellant_mass_kg > 0)
);