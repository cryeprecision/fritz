-- Add migration script here
CREATE TABLE IF NOT EXISTS "requests"
(
    "id"            INTEGER PRIMARY KEY,
    "datetime"      INTEGER NOT NULL,
    "name"          TEXT    NOT NULL,
    "url"           TEXT    NOT NULL,
    "method"        TEXT    NOT NULL,
    "duration_ms"   INTEGER NOT NULL,
    "response_code" INTEGER NULL,
    "session_id"    TEXT    NULL
);

CREATE TABLE IF NOT EXISTS "updates"
(
    "id"            INTEGER PRIMARY KEY,
    "datetime"      INTEGER NOT NULL,
    "upserted_rows" INTEGER NOT NULL
);
