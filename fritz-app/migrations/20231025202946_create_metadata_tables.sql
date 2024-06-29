-- Add migration script here
CREATE TABLE IF NOT EXISTS "requests"
(
    "id"            BIGSERIAL   PRIMARY KEY,
    "datetime"      TIMESTAMPTZ NOT NULL,
    "name"          TEXT        NOT NULL,
    "url"           TEXT        NOT NULL,
    "method"        TEXT        NOT NULL,
    "duration_ms"   BIGINT      NOT NULL,
    "response_code" BIGINT      NULL,
    "session_id"    TEXT        NULL
);

CREATE TABLE IF NOT EXISTS "updates"
(
    "id"            BIGSERIAL   PRIMARY KEY,
    "datetime"      TIMESTAMPTZ NOT NULL,
    "upserted_rows" BIGINT      NOT NULL
);
