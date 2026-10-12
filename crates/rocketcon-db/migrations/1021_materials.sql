CREATE TABLE IF NOT EXISTS materials (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    material_class TEXT NOT NULL,
    density_kg_per_m3 REAL NOT NULL CHECK (density_kg_per_m3 > 0),
    specific_heat_capacity_j_per_kg_k REAL NOT NULL CHECK (specific_heat_capacity_j_per_kg_k > 0),
    thermal_conductivity_w_per_m_k REAL NOT NULL CHECK (thermal_conductivity_w_per_m_k > 0),
    thermal_expansion_coefficient_per_k REAL NOT NULL CHECK (thermal_expansion_coefficient_per_k >= 0),
    melting_point_k REAL CHECK (melting_point_k IS NULL OR melting_point_k > 0),
    max_service_temperature_k REAL NOT NULL CHECK (max_service_temperature_k > 0),
    youngs_modulus_pa REAL NOT NULL CHECK (youngs_modulus_pa > 0),
    base_yield_strength_pa REAL NOT NULL CHECK (base_yield_strength_pa > 0),
    base_ultimate_tensile_strength_pa REAL NOT NULL CHECK (base_ultimate_tensile_strength_pa > 0),
    emissivity REAL NOT NULL CHECK (emissivity > 0.0 AND emissivity <= 1.0),
    solar_absorptivity REAL NOT NULL CHECK (solar_absorptivity > 0.0 AND solar_absorptivity <= 1.0),
    manufacturer TEXT,
    manufactured_at_unix_seconds INTEGER,
    lore_notes TEXT
);