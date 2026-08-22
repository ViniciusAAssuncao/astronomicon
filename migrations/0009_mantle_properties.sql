ALTER TABLE planets ADD COLUMN mantle_hydration_fraction REAL CHECK (mantle_hydration_fraction IS NULL OR (mantle_hydration_fraction >= 0.0 AND mantle_hydration_fraction <= 1.0));
