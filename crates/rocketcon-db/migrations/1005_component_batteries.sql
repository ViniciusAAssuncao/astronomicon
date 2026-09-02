CREATE TABLE IF NOT EXISTS component_batteries (
    component_id TEXT PRIMARY KEY NOT NULL REFERENCES components(id) ON DELETE CASCADE,
    capacity_j REAL NOT NULL CHECK (capacity_j > 0),
    max_discharge_power_w REAL NOT NULL CHECK (max_discharge_power_w > 0),
    max_charge_power_w REAL CHECK (max_charge_power_w IS NULL OR max_charge_power_w > 0)
);