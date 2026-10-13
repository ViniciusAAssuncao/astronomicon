ALTER TABLE vehicle_trajectory_patches
ADD COLUMN start_at_epoch_s REAL CHECK (start_at_epoch_s IS NULL OR start_at_epoch_s >= 0);
