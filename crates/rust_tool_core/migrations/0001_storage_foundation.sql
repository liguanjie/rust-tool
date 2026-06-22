CREATE TABLE IF NOT EXISTS app_metadata (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS app_events (
    id TEXT PRIMARY KEY NOT NULL,
    event_type TEXT NOT NULL,
    summary TEXT NOT NULL,
    payload_json TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_app_events_type_created_at
    ON app_events (event_type, created_at);

CREATE TABLE IF NOT EXISTS tool_settings (
    tool_key TEXT PRIMARY KEY NOT NULL,
    settings_json TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS export_records (
    id TEXT PRIMARY KEY NOT NULL,
    tool_key TEXT NOT NULL,
    export_type TEXT NOT NULL,
    output_path TEXT NOT NULL,
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_export_records_tool_created_at
    ON export_records (tool_key, created_at);

INSERT INTO app_metadata (key, value, updated_at)
VALUES ('schema_version', '1', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
ON CONFLICT(key) DO UPDATE SET
    value = excluded.value,
    updated_at = excluded.updated_at;

PRAGMA user_version = 1;
