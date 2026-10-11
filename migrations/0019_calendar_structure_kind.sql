ALTER TABLE calendar_definitions ADD COLUMN structure_kind TEXT NOT NULL DEFAULT 'SolarOnly' CHECK (structure_kind IN ('SolarOnly', 'LunarOnly', 'Lunisolar'));
