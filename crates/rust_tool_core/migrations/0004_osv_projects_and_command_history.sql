CREATE TABLE IF NOT EXISTS osv_projects (
    path TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    last_scanned TEXT,
    health_score INTEGER,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_osv_projects_sort_order
    ON osv_projects (sort_order, name, path);

CREATE TABLE IF NOT EXISTS osv_command_history (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,
    project_path TEXT NOT NULL,
    record_json TEXT NOT NULL,
    started_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_osv_command_history_project_started_at
    ON osv_command_history (project_path, started_at DESC);

CREATE INDEX IF NOT EXISTS idx_osv_command_history_started_at
    ON osv_command_history (started_at DESC);

INSERT INTO app_metadata (key, value, updated_at)
VALUES ('schema_version', '4', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
ON CONFLICT(key) DO UPDATE SET
    value = excluded.value,
    updated_at = excluded.updated_at;

PRAGMA user_version = 4;
