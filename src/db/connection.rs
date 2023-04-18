use anyhow::Result;

pub struct Connection {
    inner: rusqlite::Connection,
}

impl Connection {
    pub fn new() -> Result<Connection> {
        Ok(Connection {
            inner: rusqlite::Connection::open("./logs.db3")?,
        })
    }
    pub fn write_logs(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn read_logs(&self) -> Result<()> {
        unimplemented!()
    }
    fn create_tables(&self) -> Result<()> {
        self.inner.execute(
            "CREATE TABLE person (
                id    INTEGER PRIMARY KEY,
                name  TEXT NOT NULL,
                data  BLOB
            )",
            (), // empty list of parameters.
        )?;
        Ok(())
    }
}
