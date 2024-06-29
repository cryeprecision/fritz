-- Add migration script here
CREATE TABLE IF NOT EXISTS "logs"
(
    "id"                  BIGSERIAL   PRIMARY KEY,
    "datetime"            TIMESTAMPTZ NOT NULL,
    "message"             TEXT        NOT NULL,
    "message_id"          BIGINT      NOT NULL,
    "category_id"         BIGINT      NOT NULL,
    "repetition_datetime" TIMESTAMPTZ NULL,
    "repetition_count"    BIGINT      NULL,
    UNIQUE("datetime", "message_id", "category_id")
);
