ALTER TABLE planets DROP COLUMN surface_pressure_pa;
ALTER TABLE planets ADD COLUMN thermal_inertia REAL;
ALTER TABLE planets ADD COLUMN solstice_true_anomaly_rad REAL;

CREATE TABLE atmospheres (
    id TEXT PRIMARY KEY NOT NULL,
    planet_id TEXT NOT NULL UNIQUE REFERENCES planets(id) ON DELETE CASCADE,
    pressure_pa REAL NOT NULL,
    greenhouse_effect_k REAL NOT NULL,
    lapse_rate_k_per_m REAL NOT NULL
);

CREATE TABLE atmosphere_gas_components (
    atmosphere_id TEXT NOT NULL REFERENCES atmospheres(id) ON DELETE CASCADE,
    formula TEXT NOT NULL,
    percentage REAL NOT NULL,
    PRIMARY KEY (atmosphere_id, formula)
);