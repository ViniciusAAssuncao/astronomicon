CREATE TABLE IF NOT EXISTS component_solar_panels (
    component_id TEXT PRIMARY KEY NOT NULL REFERENCES components(id) ON DELETE CASCADE,
    surface_area_m2 REAL NOT NULL CHECK (surface_area_m2 > 0),
    conversion_efficiency REAL NOT NULL CHECK (conversion_efficiency > 0 AND conversion_efficiency <= 1),
    max_power_output_w REAL NOT NULL CHECK (max_power_output_w > 0),
    is_sun_tracking INTEGER NOT NULL CHECK (is_sun_tracking IN (0, 1))
);