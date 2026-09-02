ALTER TABLE vehicle_components ADD COLUMN mount_offset_x_m REAL NOT NULL DEFAULT 0.0;
ALTER TABLE vehicle_components ADD COLUMN mount_offset_y_m REAL NOT NULL DEFAULT 0.0;
ALTER TABLE vehicle_components ADD COLUMN mount_offset_z_m REAL NOT NULL DEFAULT 0.0;
ALTER TABLE vehicle_components ADD COLUMN actuation_axis_x REAL;
ALTER TABLE vehicle_components ADD COLUMN actuation_axis_y REAL;
ALTER TABLE vehicle_components ADD COLUMN actuation_axis_z REAL;
