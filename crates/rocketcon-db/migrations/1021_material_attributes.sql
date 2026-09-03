CREATE TABLE IF NOT EXISTS material_attributes (
    material_id TEXT NOT NULL REFERENCES materials(id) ON DELETE CASCADE,
    attribute_key TEXT NOT NULL,
    numeric_value REAL,
    text_value TEXT,
    PRIMARY KEY (material_id, attribute_key),
    CHECK ((numeric_value IS NOT NULL AND text_value IS NULL) OR (numeric_value IS NULL AND text_value IS NOT NULL))
);

CREATE INDEX IF NOT EXISTS idx_material_attributes_key ON material_attributes (attribute_key);
