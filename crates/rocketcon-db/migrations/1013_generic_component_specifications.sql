CREATE TABLE components_new (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    component_kind TEXT NOT NULL,
    dry_mass_kg REAL NOT NULL CHECK (dry_mass_kg > 0),
    length_m REAL NOT NULL CHECK (length_m > 0),
    diameter_m REAL NOT NULL CHECK (diameter_m > 0),
    power_consumption_w REAL NOT NULL DEFAULT 0.0 CHECK (power_consumption_w >= 0),
    manufacturer TEXT,
    manufactured_at_unix_seconds INTEGER,
    lore_notes TEXT
);

INSERT INTO components_new (id, name, component_kind, dry_mass_kg, length_m, diameter_m, power_consumption_w, manufacturer, manufactured_at_unix_seconds, lore_notes)
SELECT id, name, component_kind, dry_mass_kg, length_m, diameter_m, power_consumption_w, manufacturer, manufactured_at_unix_seconds, lore_notes FROM components;

DROP TABLE components;

ALTER TABLE components_new RENAME TO components;

CREATE TABLE IF NOT EXISTS component_attributes (
    component_id TEXT NOT NULL REFERENCES components(id) ON DELETE CASCADE,
    attribute_key TEXT NOT NULL,
    numeric_value REAL,
    text_value TEXT,
    PRIMARY KEY (component_id, attribute_key),
    CHECK ((numeric_value IS NOT NULL AND text_value IS NULL) OR (numeric_value IS NULL AND text_value IS NOT NULL))
);

INSERT INTO component_attributes (component_id, attribute_key, numeric_value, text_value)
SELECT component_id, 'fuel_propellant_id', NULL, fuel_propellant_id FROM component_engines WHERE fuel_propellant_id IS NOT NULL
UNION ALL
SELECT component_id, 'oxidizer_propellant_id', NULL, oxidizer_propellant_id FROM component_engines WHERE oxidizer_propellant_id IS NOT NULL
UNION ALL
SELECT component_id, 'specific_impulse_vacuum_s', specific_impulse_vacuum_s, NULL FROM component_engines WHERE specific_impulse_vacuum_s IS NOT NULL
UNION ALL
SELECT component_id, 'specific_impulse_sea_level_s', specific_impulse_sea_level_s, NULL FROM component_engines WHERE specific_impulse_sea_level_s IS NOT NULL
UNION ALL
SELECT component_id, 'max_thrust_n', max_thrust_n, NULL FROM component_engines WHERE max_thrust_n IS NOT NULL
UNION ALL
SELECT component_id, 'ignition_type', NULL, ignition_type FROM component_engines WHERE ignition_type IS NOT NULL
UNION ALL
SELECT component_id, 'integral_propellant_mass_kg', integral_propellant_mass_kg, NULL FROM component_engines WHERE integral_propellant_mass_kg IS NOT NULL
UNION ALL
SELECT component_id, 'max_gimbal_deflection_rad', max_gimbal_deflection_rad, NULL FROM component_engines WHERE max_gimbal_deflection_rad IS NOT NULL
UNION ALL
SELECT component_id, 'gimbal_slew_rate_rad_s', gimbal_slew_rate_rad_s, NULL FROM component_engines WHERE gimbal_slew_rate_rad_s IS NOT NULL
UNION ALL
SELECT component_id, 'min_throttle_fraction', min_throttle_fraction, NULL FROM component_engines WHERE min_throttle_fraction IS NOT NULL
UNION ALL
SELECT component_id, 'oxidizer_to_fuel_mass_ratio', oxidizer_to_fuel_mass_ratio, NULL FROM component_engines WHERE oxidizer_to_fuel_mass_ratio IS NOT NULL;

INSERT INTO component_attributes (component_id, attribute_key, numeric_value, text_value)
SELECT component_id, 'propellant_id', NULL, propellant_id FROM component_propellant_tanks WHERE propellant_id IS NOT NULL
UNION ALL
SELECT component_id, 'max_propellant_mass_kg', max_propellant_mass_kg, NULL FROM component_propellant_tanks WHERE max_propellant_mass_kg IS NOT NULL;

INSERT INTO component_attributes (component_id, attribute_key, numeric_value, text_value)
SELECT component_id, 'capacity_j', capacity_j, NULL FROM component_batteries WHERE capacity_j IS NOT NULL
UNION ALL
SELECT component_id, 'max_discharge_power_w', max_discharge_power_w, NULL FROM component_batteries WHERE max_discharge_power_w IS NOT NULL
UNION ALL
SELECT component_id, 'max_charge_power_w', max_charge_power_w, NULL FROM component_batteries WHERE max_charge_power_w IS NOT NULL;

INSERT INTO component_attributes (component_id, attribute_key, numeric_value, text_value)
SELECT component_id, 'surface_area_m2', surface_area_m2, NULL FROM component_solar_panels WHERE surface_area_m2 IS NOT NULL
UNION ALL
SELECT component_id, 'conversion_efficiency', conversion_efficiency, NULL FROM component_solar_panels WHERE conversion_efficiency IS NOT NULL
UNION ALL
SELECT component_id, 'max_power_output_w', max_power_output_w, NULL FROM component_solar_panels WHERE max_power_output_w IS NOT NULL
UNION ALL
SELECT component_id, 'is_sun_tracking', CAST(is_sun_tracking AS REAL), NULL FROM component_solar_panels WHERE is_sun_tracking IS NOT NULL;

INSERT INTO component_attributes (component_id, attribute_key, numeric_value, text_value)
SELECT component_id, 'propellant_id', NULL, propellant_id FROM component_reaction_control_thrusters WHERE propellant_id IS NOT NULL
UNION ALL
SELECT component_id, 'specific_impulse_vacuum_s', specific_impulse_vacuum_s, NULL FROM component_reaction_control_thrusters WHERE specific_impulse_vacuum_s IS NOT NULL
UNION ALL
SELECT component_id, 'max_thrust_n', max_thrust_n, NULL FROM component_reaction_control_thrusters WHERE max_thrust_n IS NOT NULL
UNION ALL
SELECT component_id, 'min_impulse_bit_n_s', min_impulse_bit_n_s, NULL FROM component_reaction_control_thrusters WHERE min_impulse_bit_n_s IS NOT NULL;

INSERT INTO component_attributes (component_id, attribute_key, numeric_value, text_value)
SELECT component_id, 'max_torque_n_m', max_torque_n_m, NULL FROM component_reaction_wheels WHERE max_torque_n_m IS NOT NULL
UNION ALL
SELECT component_id, 'max_angular_momentum_storage_n_m_s', max_angular_momentum_storage_n_m_s, NULL FROM component_reaction_wheels WHERE max_angular_momentum_storage_n_m_s IS NOT NULL;

DROP TABLE IF EXISTS component_engines;
DROP TABLE IF EXISTS component_propellant_tanks;
DROP TABLE IF EXISTS component_batteries;
DROP TABLE IF EXISTS component_solar_panels;
DROP TABLE IF EXISTS component_reaction_control_thrusters;
DROP TABLE IF EXISTS component_reaction_wheels;
