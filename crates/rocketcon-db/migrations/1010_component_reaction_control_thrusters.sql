CREATE TABLE IF NOT EXISTS component_reaction_control_thrusters (
    component_id TEXT PRIMARY KEY NOT NULL REFERENCES components(id) ON DELETE CASCADE,
    propellant_id TEXT NOT NULL REFERENCES propellants(id) ON DELETE RESTRICT,
    specific_impulse_vacuum_s REAL NOT NULL CHECK (specific_impulse_vacuum_s > 0),
    max_thrust_n REAL NOT NULL CHECK (max_thrust_n > 0),
    min_impulse_bit_n_s REAL CHECK (min_impulse_bit_n_s IS NULL OR min_impulse_bit_n_s > 0)
);