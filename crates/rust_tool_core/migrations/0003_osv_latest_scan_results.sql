CREATE TABLE IF NOT EXISTS osv_latest_scan_results (
    project_path TEXT PRIMARY KEY NOT NULL,
    result_json TEXT NOT NULL,
    scanned_at TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_osv_latest_scan_results_scanned_at
    ON osv_latest_scan_results (scanned_at DESC);

INSERT INTO app_metadata (key, value, updated_at)
VALUES ('schema_version', '3', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
ON CONFLICT(key) DO UPDATE SET
    value = excluded.value,
    updated_at = excluded.updated_at;

PRAGMA user_version = 3;
