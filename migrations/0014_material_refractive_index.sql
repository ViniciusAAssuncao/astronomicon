ALTER TABLE material_properties ADD COLUMN refractive_index_real REAL NOT NULL DEFAULT 1.5 CHECK (refractive_index_real >= 1.0);
ALTER TABLE material_properties ADD COLUMN refractive_index_imag REAL NOT NULL DEFAULT 0.005 CHECK (refractive_index_imag >= 0.0);

UPDATE material_properties SET refractive_index_real = 1.55, refractive_index_imag = 0.005 WHERE name = 'Silicate Rock';
UPDATE material_properties SET refractive_index_real = 1.31, refractive_index_imag = 0.00000001 WHERE name = 'Water Ice';
UPDATE material_properties SET refractive_index_real = 1.65, refractive_index_imag = 0.1 WHERE name = 'Carbonaceous Rock';
UPDATE material_properties SET refractive_index_real = 2.5, refractive_index_imag = 3.0 WHERE name = 'Iron-Nickel';