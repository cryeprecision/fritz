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
    fn new_entries<'a>(latest: &LogEntry, new_entries: &'a [LogEntry]) -> &'a [LogEntry] {
        debug_assert!(new_entries.windows(2).all(|e| e[0].time <= e[1].time));

        if new_entries.first().unwrap().time > latest.time {
            // oldest new entry is already new, use all entries
            return new_entries;
        }
        if new_entries.last().unwrap().time < latest.time {
            // newest new entry is old, don't use any of them
            return &[];
        }

        // figure out where the cut is for new entries and only use the new ones

        // ['<', '=', '=', '=', '>']
        //        ^              ^---- `eq_end`
        //        |------------------- `eq_begin`
        //
        // ['=', '=', '=', '=']
        //   ^                 ^---- `eq_end`
        //   |---------------------- `eq_begin`
        //
        // ['<', '<', '>', '>']
        //             ^------- `eq_end`
        //             |------- `eq_begin`

        let eq_begin = new_entries.partition_point(|log| log.time < latest.time);
        let eq_end = new_entries.partition_point(|log| log.time <= latest.time);

        if eq_begin == eq_end {
            // no elements have the same timestamp
            return &new_entries[eq_begin..];
        }

        let last_old_index = (eq_begin..eq_end)
            .rev()
            .find(|&i| new_entries[i].msg_id == latest.msg_id);

        match last_old_index {
            Some(index) => {
                if index == new_entries.len() - 1 {
                    &[]
                } else {
                    &new_entries[(index + 1)..]
                }
            }
            None => &new_entries[eq_begin..],
        }
    }

    pub fn append_logs(&self, entries: &[LogEntry]) -> Result<()> {
        // TODO: replace this with `is_sorted` once that is stable
        assert!(entries.windows(2).all(|e| e[0].time <= e[1].time));

        let latest = self.latest_logs(Some(1))?.into_iter().next();
        match latest {
            Some(latest) => {
                let new_entries = Self::new_entries(&latest, entries);
                self.append_logs_impl(new_entries)
            }
            None => self.append_logs_impl(entries),
        }
    }

    pub fn latest_logs(&self, limit: Option<i64>) -> Result<Vec<LogEntry>> {
        let limit = limit.unwrap_or(i64::MAX).max(1);
        let mut stmt = self.inner.prepare(
            "SELECT message, message_id, category, logged_at
                FROM logs
                ORDER BY id DESC
                LIMIT ?1",
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

    pub fn newest_logs(&self, limit: Option<i64>) -> Result<Vec<LogEntry>> {
        let limit = limit.unwrap_or(i64::MAX).max(1);
        let mut stmt = self.inner.prepare(
            "SELECT message, message_id, category, logged_at
                FROM logs
                ORDER BY id ASC
                LIMIT ?1",
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

    macro_rules! entry {
        ($id:literal, $timestamp:literal) => {
            LogEntry::new(format!("msg {}", $timestamp), $id, 1, $timestamp)
        };
    }
    macro_rules! entries_equal {
        ($lhs:ident, $rhs:ident) => {
            assert_eq!($lhs.len(), $rhs.len());
            for (lhs, rhs) in $lhs.iter().zip($rhs.iter()) {
                assert_eq!(lhs, rhs);
            }
        };
    }

    #[test]
    fn logs_insert_logic_wrong_order() {
        let db = Connection::open_in_memory().unwrap();
        db.create_logs_table().unwrap();

        // `example_logs.json` and `example_logs_2.json` are disjunct
        // and `example_logs_2.json` contains newer logs

        let data_1 = std::fs::read_to_string("./example_logs.json").unwrap();
        let parsed_1 = LogEntry::from_json(&data_1).unwrap();

        let data_2 = std::fs::read_to_string("./example_logs_2.json").unwrap();
        let parsed_2 = LogEntry::from_json(&data_2).unwrap();

        db.append_logs(&parsed_2).unwrap();
        db.append_logs(&parsed_1).unwrap();
        let fetched = db.newest_logs(None).unwrap();

        // should only contain entries from `example_logs_2.json`
        // because the old logs from `example_logs.json` should be rejected
        // since they are older
        entries_equal!(parsed_2, fetched);
    }

    #[test]
    fn test_insert_logic_correct_order() {
        let db = Connection::open_in_memory().unwrap();
        db.create_logs_table().unwrap();

        // `example_logs.json` and `example_logs_2.json` are disjunct
        // and `example_logs_2.json` contains newer logs

        let data_1 = std::fs::read_to_string("./example_logs.json").unwrap();
        let parsed_1 = LogEntry::from_json(&data_1).unwrap();

        let data_2 = std::fs::read_to_string("./example_logs_2.json").unwrap();
        let parsed_2 = LogEntry::from_json(&data_2).unwrap();

        db.append_logs(&parsed_1).unwrap();
        db.append_logs(&parsed_2).unwrap();
        let fetched = db.newest_logs(None).unwrap();

        let mut parsed_combined = parsed_1.clone();
        parsed_combined.extend_from_slice(&parsed_2);

        // should contain all entries since they were now inserted
        // in the correct oder
        entries_equal!(fetched, parsed_combined);
    }

    #[test]
    fn new_entries_regular() {
        let latest = entry!(1, 1);
        let entries = vec![entry!(1, 2), entry!(1, 3), entry!(1, 4)];

        let new_entries = Connection::new_entries(&latest, &entries);
        assert_eq!(new_entries, entries);
    }

    #[test]
    fn new_entries_all_old() {
        let latest = entry!(1, 3);
        let entries = vec![entry!(1, 0), entry!(1, 1), entry!(1, 2)];

        let new_entries = Connection::new_entries(&latest, &entries);
        assert_eq!(new_entries, &[]);
    }

    #[test]
    fn new_entries_some_old() {
        let latest = entry!(1, 0);
        let entries = vec![entry!(1, 0), entry!(1, 1), entry!(1, 2)];

        let new_entries = Connection::new_entries(&latest, &entries);
        assert_eq!(new_entries, &entries[1..]);
    }

    #[test]
    fn new_entries_some_old_2() {
        let latest = entry!(1, 1);
        let entries = vec![entry!(0, 0), entry!(2, 1), entry!(2, 2)];

        let new_entries = Connection::new_entries(&latest, &entries);
        assert_eq!(new_entries, &entries[1..]);
    }
}
