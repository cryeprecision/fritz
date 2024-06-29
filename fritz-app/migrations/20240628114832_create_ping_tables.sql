-- Add migration script here
CREATE TABLE IF NOT EXISTS "ping"
(
    "id"          BIGSERIAL   PRIMARY KEY,
    "datetime"    TIMESTAMPTZ NOT NULL,
    "target"      TEXT        NOT NULL,
    "duration_ms" BIGINT      NULL,
    "ttl"         BIGINT      NULL,
    "bytes"       BIGINT      NULL
);
