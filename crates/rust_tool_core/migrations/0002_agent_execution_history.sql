CREATE TABLE IF NOT EXISTS agent_execution_history (
    id TEXT PRIMARY KEY NOT NULL,
    script_name TEXT NOT NULL,
    args TEXT NOT NULL,
    exit_code INTEGER NOT NULL,
    success INTEGER NOT NULL CHECK (success IN (0, 1)),
    stdout TEXT NOT NULL,
    stderr TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_agent_execution_history_script_timestamp
    ON agent_execution_history (script_name, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_agent_execution_history_timestamp
    ON agent_execution_history (timestamp DESC);

INSERT INTO app_metadata (key, value, updated_at)
VALUES ('schema_version', '2', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
ON CONFLICT(key) DO UPDATE SET
    value = excluded.value,
    updated_at = excluded.updated_at;

PRAGMA user_version = 2;
