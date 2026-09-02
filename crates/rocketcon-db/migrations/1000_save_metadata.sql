CREATE TABLE IF NOT EXISTS save_metadata (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    save_uuid TEXT NOT NULL,
    created_at_unix_seconds INTEGER NOT NULL,
    source_template_path TEXT NOT NULL
);