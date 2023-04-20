use std::path::Path;

use rusqlite::{params, Result};

use crate::logs::LogEntry;

pub struct Connection {
    inner: rusqlite::Connection,
}

impl Connection {
    pub fn open(path: impl AsRef<Path>) -> Result<Connection> {
        Ok(Connection {
            inner: rusqlite::Connection::open(path.as_ref())?,
        })
    }
    pub fn open_in_memory() -> Result<Connection> {
        Ok(Connection {
            inner: rusqlite::Connection::open_in_memory()?,
        })
    }
    fn append_logs_impl(&self, entries: &[LogEntry]) -> Result<()> {
        let mut stmt = self.inner.prepare(
            "INSERT INTO logs (message, message_id, category, logged_at)
                VALUES (?1, ?2, ?3, ?4)",
        )?;
        for entry in entries {
            stmt.execute(params![
                entry.raw_msg.as_str(),
                entry.msg_id,
                entry.msg.category(),
                entry.time.timestamp()
            ])?;
        }
        Ok(())
    }
    pub fn append_logs(&self, entries: &[LogEntry]) -> Result<()> {
        fn new_entries<'a>(con: &Connection, entries: &'a [LogEntry]) -> Result<&'a [LogEntry]> {
            let latest = match con.latest_log()? {
                None => {
                    // the database is empty, use all entries
                    return Ok(entries);
                }
                Some(latest) => latest,
            };

            if entries.first().unwrap().time > latest.time {
                // oldest new entry is already new, use all entries
                return Ok(entries);
            }
            if entries.last().unwrap().time < latest.time {
                // newest new entry is old, don't use any of them
                return Ok(&[]);
            }

            // figure out where the cut is for new entries and only use the new ones

            // ['<', '=', '=', '=', '>']
            //        ^              ^---- `eq_end`
            //        |------------------- `eq_begin`
            //
            // also possible:
            // ['=', '=', '=', '=']
            //   ^                 ^---- `eq_end`
            //   |---------------------- `eq_begin`
            let eq_begin = entries.partition_point(|log| log.time < latest.time);
            let eq_end = entries.partition_point(|log| log.time <= latest.time);

            let last_old_index =
                match entries.iter().enumerate().position(|(i, log)| {
                    (i >= eq_begin && i < eq_end) && log.msg_id == latest.msg_id
                }) {
                    None => {
                        // since we assume that the combination of message_id and timestamp
                        // is unique, that means all logs with the same timestamp (or newer)
                        // are actually new
                        return Ok(&entries[eq_begin..]);
                    }
                    Some(index) => index,
                };

            Ok(&entries[last_old_index..])
        }

        // TODO: replace this with `is_sorted` once that is stable
        assert!(entries.windows(2).all(|e| e[0].time <= e[1].time));

        let new_entries = new_entries(self, entries)?;
        self.append_logs_impl(new_entries)
    }

    pub fn latest_log(&self) -> Result<Option<LogEntry>> {
        unimplemented!()
    }

    pub fn read_logs(&self, limit: Option<i64>) -> Result<Vec<LogEntry>> {
        let limit = limit.unwrap_or(i64::MAX).max(1);
        let mut stmt = self.inner.prepare(
            "SELECT message, message_id, category, logged_at
                FROM logs
                LIMIT ?1
                ORDER BY id ASC",
        )?;
        let entries = stmt.query_map(params![limit], |row| {
            Ok(LogEntry::new(
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
            ))
        })?;
        entries.collect()
    }

    fn create_logs_table(&self) -> Result<()> {
        // create the main table structure
        self.inner.execute(
            "CREATE TABLE IF NOT EXISTS logs(
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                message    VARCHAR NOT NULL,
                message_id INTEGER NOT NULL,
                category   INTEGER NOT NULL,
                logged_at  INTEGER NOT NULL
            )",
            (),
        )?;
        // add an index for fast lookup of logs by date
        self.inner.execute(
            "CREATE INDEX IF NOT EXISTS logs_logged_at_index
                ON logs (logged_at DESC)",
            (),
        )?;
        // the combination of `logged_at` and `message_id` must be unique
        self.inner.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS logs_unique_index
                ON logs (logged_at DESC, message_id)",
            (),
        )?;
        Ok(())
    }
    fn drop_logs_table(&self) -> Result<()> {
        self.inner.execute("DROP TABLE IF EXISTS logs", ())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::logs::LogEntry;

    use super::Connection;

    #[test]
    fn tabless() {
        let db = Connection::open("./test_logs.db3").unwrap();
        db.drop_logs_table().unwrap();
        db.create_logs_table().unwrap();

        let data = std::fs::read_to_string("./example_logs.json").unwrap();
        let parsed = LogEntry::from_json(&data).unwrap();

        db.append_logs(&parsed).unwrap();

        let fetched = db.read_logs(None).unwrap();
        for (fetched, parsed) in fetched.iter().zip(parsed.iter()) {
            assert_eq!(fetched, parsed, "fetched != parsed");
        }
    }

    #[test]
    fn tables() {
        let db = Connection::open_in_memory().unwrap();
        db.create_logs_table().unwrap();

        let data = std::fs::read_to_string("./example_logs.json").unwrap();
        let parsed = LogEntry::from_json(&data).unwrap();

        db.append_logs(&parsed).unwrap();
        let fetched = db.read_logs(None).unwrap();

        assert_eq!(fetched, parsed);
    }
}
