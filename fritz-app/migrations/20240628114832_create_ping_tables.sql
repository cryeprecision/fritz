-- Add migration script here
CREATE TABLE IF NOT EXISTS "ping"
(
    "id"            INTEGER PRIMARY KEY,
    "datetime"      INTEGER NOT NULL,
    "target"        TEXT    NOT NULL,
    "duration_ms"   INTEGER NULL,
    "ttl"           INTEGER NULL,
    "bytes"         INTEGER NULL
);
