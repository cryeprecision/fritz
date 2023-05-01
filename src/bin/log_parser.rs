use anyhow::Context;
use fritz_log_parser::{logger, Connection};

fn main() {
    logger::init()
        .context("couldn't initialize logger")
        .unwrap();

    let _db = Connection::open("./logs.db3")
        .context("couldn't open logs database file")
        .unwrap();
}
