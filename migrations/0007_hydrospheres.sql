ALTER TABLE planets DROP COLUMN hydrosphere_fraction;

CREATE TABLE hydrospheres (
    id TEXT PRIMARY KEY NOT NULL,
    planet_id TEXT NOT NULL UNIQUE REFERENCES planets(id) ON DELETE CASCADE,
    average_depth_m REAL NOT NULL,
    surface_coverage_fraction REAL NOT NULL,
    salinity_or_solute_mass_fraction REAL NOT NULL
);

CREATE TABLE hydrosphere_components (
    hydrosphere_id TEXT NOT NULL REFERENCES hydrospheres(id) ON DELETE CASCADE,
    formula TEXT NOT NULL,
    percentage REAL NOT NULL,
    PRIMARY KEY (hydrosphere_id, formula)
);