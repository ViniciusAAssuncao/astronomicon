ALTER TABLE atmospheres ADD COLUMN surface_humidity REAL CHECK (surface_humidity IS NULL OR (surface_humidity >= 0.0 AND surface_humidity <= 1.0));
ALTER TABLE atmospheres ADD COLUMN cloud_coverage_fraction REAL CHECK (cloud_coverage_fraction IS NULL OR (cloud_coverage_fraction >= 0.0 AND cloud_coverage_fraction <= 1.0));
ALTER TABLE planets ADD COLUMN dust_availability_factor REAL CHECK (dust_availability_factor IS NULL OR (dust_availability_factor >= 0.0 AND dust_availability_factor <= 1.0));
