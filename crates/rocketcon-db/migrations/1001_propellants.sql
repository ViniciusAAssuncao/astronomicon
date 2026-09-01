CREATE TABLE IF NOT EXISTS propellants (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    propellant_kind TEXT NOT NULL CHECK (propellant_kind IN ('LiquidFuel', 'LiquidOxidizer', 'SolidPropellant', 'Monopropellant', 'NobleGasPropellant', 'ReactionMass')),
    chemical_formula TEXT,
    density_kg_per_m3 REAL NOT NULL CHECK (density_kg_per_m3 > 0),
    is_cryogenic INTEGER NOT NULL CHECK (is_cryogenic IN (0, 1)),
    is_hypergolic INTEGER NOT NULL CHECK (is_hypergolic IN (0, 1))
);