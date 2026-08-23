CREATE TABLE IF NOT EXISTS minor_planets (
    id TEXT PRIMARY KEY NOT NULL,
    star_system_id TEXT REFERENCES star_systems(id) ON DELETE CASCADE,
    parent_star_id TEXT REFERENCES stars(id) ON DELETE RESTRICT,
    parent_planet_id TEXT REFERENCES planets(id) ON DELETE RESTRICT,
    parent_barycenter_id TEXT REFERENCES barycenters(id) ON DELETE RESTRICT,
    parent_minor_planet_id TEXT REFERENCES minor_planets(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    spectral_type TEXT NOT NULL CHECK (spectral_type IN ('C', 'S', 'M', 'D', 'V', 'P')),
    mass_kg REAL NOT NULL CHECK (mass_kg > 0.0),
    axis_a_m REAL CHECK (axis_a_m IS NULL OR axis_a_m > 0.0),
    axis_b_m REAL CHECK (axis_b_m IS NULL OR axis_b_m > 0.0),
    axis_c_m REAL CHECK (axis_c_m IS NULL OR axis_c_m > 0.0),
    rotation_period_s REAL CHECK (rotation_period_s IS NULL OR rotation_period_s > 0.0),
    axial_tilt_rad REAL,
    macroporosity REAL CHECK (macroporosity IS NULL OR (macroporosity >= 0.0 AND macroporosity <= 1.0)),
    geometric_albedo REAL CHECK (geometric_albedo IS NULL OR (geometric_albedo >= 0.0 AND geometric_albedo <= 1.0)),
    bond_albedo REAL CHECK (bond_albedo IS NULL OR (bond_albedo >= 0.0 AND bond_albedo <= 1.0)),
    semi_major_axis_m REAL,
    eccentricity REAL CHECK (eccentricity IS NULL OR (eccentricity >= 0.0 AND eccentricity < 1.0)),
    inclination_rad REAL,
    longitude_ascending_node_rad REAL,
    argument_periapsis_rad REAL,
    mean_anomaly_at_epoch_rad REAL,
    CHECK (
        (CASE WHEN parent_star_id IS NOT NULL THEN 1 ELSE 0 END +
         CASE WHEN parent_planet_id IS NOT NULL THEN 1 ELSE 0 END +
         CASE WHEN parent_barycenter_id IS NOT NULL THEN 1 ELSE 0 END +
         CASE WHEN parent_minor_planet_id IS NOT NULL THEN 1 ELSE 0 END) <= 1
    ),
    CHECK (
        (axis_a_m IS NULL AND axis_b_m IS NULL AND axis_c_m IS NULL) OR
        (axis_a_m >= axis_b_m AND axis_b_m >= axis_c_m)
    )
);

ALTER TABLE stars ADD COLUMN parent_minor_planet_id TEXT REFERENCES minor_planets(id) ON DELETE RESTRICT;
ALTER TABLE planets ADD COLUMN parent_minor_planet_id TEXT REFERENCES minor_planets(id) ON DELETE RESTRICT;
ALTER TABLE barycenters ADD COLUMN parent_minor_planet_id TEXT REFERENCES minor_planets(id) ON DELETE RESTRICT;