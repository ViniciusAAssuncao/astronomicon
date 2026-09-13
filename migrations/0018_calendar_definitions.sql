CREATE TABLE IF NOT EXISTS calendar_definitions (
    id TEXT PRIMARY KEY NOT NULL,
    planet_id TEXT NOT NULL REFERENCES planets(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    epoch_seconds_since_j2000 REAL NOT NULL,
    day_convention TEXT NOT NULL CHECK (day_convention IN ('Solar', 'Sidereal')),
    year_convention TEXT NOT NULL CHECK (year_convention IN ('Sidereal', 'Tropical', 'Anomalistic')),
    reference_moon_planet_id TEXT REFERENCES planets(id) ON DELETE RESTRICT,
    reference_moon_minor_planet_id TEXT REFERENCES minor_planets(id) ON DELETE RESTRICT,
    founding_event_description TEXT,
    CHECK (
        (CASE WHEN reference_moon_planet_id IS NOT NULL THEN 1 ELSE 0 END +
         CASE WHEN reference_moon_minor_planet_id IS NOT NULL THEN 1 ELSE 0 END) <= 1
    )
);

CREATE TABLE IF NOT EXISTS calendar_tracked_moons (
    id TEXT PRIMARY KEY NOT NULL,
    calendar_id TEXT NOT NULL REFERENCES calendar_definitions(id) ON DELETE CASCADE,
    moon_planet_id TEXT REFERENCES planets(id) ON DELETE RESTRICT,
    moon_minor_planet_id TEXT REFERENCES minor_planets(id) ON DELETE RESTRICT,
    CHECK (
        (CASE WHEN moon_planet_id IS NOT NULL THEN 1 ELSE 0 END +
         CASE WHEN moon_minor_planet_id IS NOT NULL THEN 1 ELSE 0 END) = 1
    )
);