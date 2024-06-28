-- Add migration script here
CREATE TABLE IF NOT EXISTS "logs"
(
    "id"                  INTEGER PRIMARY KEY,
    "datetime"            INTEGER NOT NULL,
    "message"             TEXT    NOT NULL,
    "message_id"          INTEGER NOT NULL,
    "category_id"         INTEGER NOT NULL,
    "repetition_datetime" INTEGER NULL,
    "repetition_count"    INTEGER NULL,
    UNIQUE(datetime, message_id, category_id)
);
