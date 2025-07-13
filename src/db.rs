use rusqlite::{Connection, Result};

pub fn initialize_database() -> Result<()> {
    let conn = Connection::open("bipa.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS nodes (
            public_key TEXT PRIMARY KEY,
            alias TEXT NOT NULL,
            capacity INTEGER NOT NULL,
            first_seen INTEGER NOT NULL
        )",
        [],
    )?;
    Ok(())
} 