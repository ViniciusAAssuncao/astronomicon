CREATE TABLE material_properties (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    density_kg_per_m3 REAL NOT NULL CHECK (density_kg_per_m3 > 0.0),
    shear_modulus_pa REAL NOT NULL CHECK (shear_modulus_pa > 0.0),
    base_yield_stress_pa REAL NOT NULL CHECK (base_yield_stress_pa > 0.0),
    thermal_conductivity_w_per_m_k REAL NOT NULL CHECK (thermal_conductivity_w_per_m_k > 0.0),
    specific_heat_capacity_j_per_kg_k REAL NOT NULL CHECK (specific_heat_capacity_j_per_kg_k > 0.0),
    thermal_expansion_per_k REAL NOT NULL CHECK (thermal_expansion_per_k > 0.0),
    solidus_temperature_k REAL NOT NULL CHECK (solidus_temperature_k > 0.0),
    liquidus_temperature_k REAL NOT NULL CHECK (liquidus_temperature_k >= solidus_temperature_k)
);

INSERT INTO material_properties (
    id, name, density_kg_per_m3, shear_modulus_pa, base_yield_stress_pa,
    thermal_conductivity_w_per_m_k, specific_heat_capacity_j_per_kg_k,
    thermal_expansion_per_k, solidus_temperature_k, liquidus_temperature_k
) VALUES
    ('c8b0e7a1-8d2a-4c28-98e6-238d99c43d01', 'Silicate Rock', 3300.0, 3.0e10, 1.0e8, 2.5, 1000.0, 3.0e-5, 1400.0, 1800.0),
    ('c8b0e7a1-8d2a-4c28-98e6-238d99c43d02', 'Water Ice', 917.0, 3.5e9, 1.0e6, 2.2, 2050.0, 1.5e-4, 270.0, 273.15),
    ('c8b0e7a1-8d2a-4c28-98e6-238d99c43d03', 'Carbonaceous Rock', 2900.0, 4.0e10, 1.5e8, 3.0, 950.0, 2.0e-5, 1600.0, 2000.0),
    ('c8b0e7a1-8d2a-4c28-98e6-238d99c43d04', 'Iron-Nickel', 7800.0, 8.0e10, 3.0e8, 40.0, 450.0, 1.2e-5, 1800.0, 2100.0);

CREATE TABLE planet_lithosphere_components (
    planet_id TEXT NOT NULL REFERENCES planets(id) ON DELETE CASCADE,
    material_id TEXT NOT NULL REFERENCES material_properties(id) ON DELETE RESTRICT,
    percentage REAL NOT NULL CHECK (percentage >= 0.0 AND percentage <= 100.0),
    PRIMARY KEY (planet_id, material_id)
);
