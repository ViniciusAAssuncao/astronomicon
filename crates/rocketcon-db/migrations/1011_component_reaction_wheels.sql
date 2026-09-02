CREATE TABLE IF NOT EXISTS component_reaction_wheels (
    component_id TEXT PRIMARY KEY NOT NULL REFERENCES components(id) ON DELETE CASCADE,
    max_torque_n_m REAL NOT NULL CHECK (max_torque_n_m > 0),
    max_angular_momentum_storage_n_m_s REAL NOT NULL CHECK (max_angular_momentum_storage_n_m_s > 0)
);
