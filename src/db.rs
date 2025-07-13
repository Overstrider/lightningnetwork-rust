use rusqlite::{Connection, Result};

pub fn initialize_database() -> Result<()> {
    let conn = Connection::open("bipa.db")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS nodes (
            id            INTEGER PRIMARY KEY,
            public_key    TEXT NOT NULL UNIQUE,
            alias         TEXT NOT NULL,
            capacity      INTEGER NOT NULL,
            first_seen    TEXT NOT NULL
        )",
        (),
    )?;

    Ok(())
} 