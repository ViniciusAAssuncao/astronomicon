CREATE TABLE IF NOT EXISTS universe_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    seconds_since_j2000_epoch REAL NOT NULL
);

CREATE TABLE IF NOT EXISTS star_systems (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    right_ascension_rad REAL,
    declination_rad REAL,
    distance_from_sol_m REAL
);

CREATE TABLE IF NOT EXISTS stars (
    id TEXT PRIMARY KEY,
    star_system_id TEXT REFERENCES star_systems(id),
    name TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('Star', 'WhiteDwarf', 'NeutronStar', 'BlackHole', 'BrownDwarf', 'Exotic')),
    mass_kg REAL NOT NULL CHECK (mass_kg > 0.0),
    radius_m REAL CHECK (radius_m IS NULL OR radius_m > 0.0),
    effective_temperature_k REAL CHECK (effective_temperature_k IS NULL OR effective_temperature_k > 0.0),
    rotation_period_s REAL CHECK (rotation_period_s IS NULL OR rotation_period_s > 0.0),
    axial_tilt_rad REAL,
    semi_major_axis_m REAL,
    eccentricity REAL CHECK (eccentricity IS NULL OR (eccentricity >= 0.0 AND eccentricity < 1.0)),
    inclination_rad REAL,
    longitude_ascending_node_rad REAL,
    argument_periapsis_rad REAL,
    mean_anomaly_at_epoch_rad REAL
);

CREATE TABLE IF NOT EXISTS planets (
    id TEXT PRIMARY KEY,
    parent_star_id TEXT REFERENCES stars(id),
    parent_planet_id TEXT REFERENCES planets(id),
    name TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('Telluric', 'GasGiant', 'IceGiant', 'DwarfPlanet', 'Chthonian', 'CarbonPlanet', 'IcyBody', 'Exotic')),
    mass_kg REAL NOT NULL CHECK (mass_kg > 0.0),
    equatorial_radius_m REAL CHECK (equatorial_radius_m IS NULL OR equatorial_radius_m > 0.0),
    polar_radius_m REAL CHECK (polar_radius_m IS NULL OR polar_radius_m > 0.0),
    rotation_period_s REAL CHECK (rotation_period_s IS NULL OR rotation_period_s > 0.0),
    axial_tilt_rad REAL,
    geometric_albedo REAL CHECK (geometric_albedo IS NULL OR (geometric_albedo >= 0.0 AND geometric_albedo <= 1.0)),
    bond_albedo REAL CHECK (bond_albedo IS NULL OR (bond_albedo >= 0.0 AND bond_albedo <= 1.0)),
    surface_pressure_pa REAL CHECK (surface_pressure_pa IS NULL OR surface_pressure_pa >= 0.0),
    semi_major_axis_m REAL,
    eccentricity REAL CHECK (eccentricity IS NULL OR (eccentricity >= 0.0 AND eccentricity < 1.0)),
    inclination_rad REAL,
    longitude_ascending_node_rad REAL,
    argument_periapsis_rad REAL,
    mean_anomaly_at_epoch_rad REAL,
    CHECK (NOT (parent_star_id IS NOT NULL AND parent_planet_id IS NOT NULL))
);
