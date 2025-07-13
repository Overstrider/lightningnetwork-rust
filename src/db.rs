use rusqlite::{Connection, Result, OpenFlags};
use chrono::DateTime;
use log::{error, info};

// This module handles all the database setup and migration logic.

/// Checks if we need to update the database schema.
/// The old schema used TEXT for `first_seen`, but the new one uses INTEGER.
fn needs_migration(conn: &Connection) -> Result<bool> {
    let mut stmt = conn.prepare("PRAGMA table_info(nodes)")?;
    let column_types: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(1)?, row.get(2)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    // If the 'first_seen' column is TEXT, we need to migrate.
    if let Some((_, col_type)) = column_types.iter().find(|(name, _)| name == "first_seen") {
        if col_type.eq_ignore_ascii_case("TEXT") {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Updates the database from the old schema to the new one.
/// It renames the old table, creates a new one, and copies the data over,
/// converting `first_seen` from text to a number.
/// It's all in a transaction, so it's safe.
fn run_migration(conn: &mut Connection) -> Result<()> {
    info!("[DB] Old schema found, running migration...");    
    
    let tx = conn.transaction()?;

    // 1. Rename the old table so we don't lose data.
    tx.execute("ALTER TABLE nodes RENAME TO nodes_old_migration_temp", [])?;

    // 2. Create the new table with the correct schema.
    tx.execute(
        "CREATE TABLE nodes (
            public_key    TEXT PRIMARY KEY,
            alias         TEXT NOT NULL,
            capacity      INTEGER NOT NULL,
            first_seen    INTEGER NOT NULL
        )",
        [],
    )?;

    // 3. Copy data from the old table to the new one.
    {
        // A temporary struct to hold data from the old table format.
        struct OldNode {
            public_key: String,
            alias: String,
            capacity: i64,
            first_seen: String,
        }

        let mut select_stmt = tx.prepare("SELECT public_key, alias, capacity, first_seen FROM nodes_old_migration_temp")?;
        let old_nodes_iter = select_stmt.query_map([], |row| {
            Ok(OldNode {
                public_key: row.get(0)?,
                alias: row.get(1)?,
                capacity: row.get(2)?,
                first_seen: row.get(3)?,
            })
        })?;

        for node_result in old_nodes_iter {
            let old_node = node_result?;
            // Convert the old date string to a Unix timestamp.
            // If it fails, just use 0 and log an error.
            let first_seen_ts = DateTime::parse_from_rfc3339(&old_node.first_seen)
                                    .map(|dt| dt.timestamp())
                                    .unwrap_or_else(|e| {
                                        error!("Failed to parse date '{}': {}. Defaulting to 0.", old_node.first_seen, e);
                                        0
                                    }); 

            tx.execute(
                "INSERT OR IGNORE INTO nodes (public_key, alias, capacity, first_seen) VALUES (?1, ?2, ?3, ?4)",
                (&old_node.public_key, &old_node.alias, &old_node.capacity, &first_seen_ts),
            )?;
        }
    }
    
    // 4. Clean up the old table.
    tx.execute("DROP TABLE nodes_old_migration_temp", [])?;

    // 5. Commit everything.
    tx.commit()?;
    info!("[DB] Migration finished.");
    Ok(())
}


/// Gets the database ready to use.
/// It creates the DB file and the `nodes` table if they don't exist.
/// If the table is old, it runs the migration.
pub fn initialize_database(db_path: &str) -> Result<()> {
    // Open the DB connection.
    // We set a busy timeout just in case the database is locked for a moment.
    let mut conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_NO_MUTEX).map(|c| {
        c.busy_timeout(std::time::Duration::from_secs(5));
        c
    })?;

    // Check if the 'nodes' table already exists.
    let table_exists: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type='table' AND name='nodes')",
        [],
        |row| row.get(0),
    )?;

    if !table_exists {
        info!("[DB] 'nodes' table not found, creating it.");
        conn.execute(
            "CREATE TABLE nodes (
                public_key    TEXT PRIMARY KEY,
                alias         TEXT NOT NULL,
                capacity      INTEGER NOT NULL,
                first_seen    INTEGER NOT NULL
            )",
            [],
        )?;
        // Add an index to make sorting by capacity faster.
        conn.execute("CREATE INDEX IF NOT EXISTS idx_capacity ON nodes(capacity DESC)", [])?;
    } else {
        // If the table exists, check if we need to update its schema.
        if needs_migration(&conn)? {
            run_migration(&mut conn)?;
        }
    }

    Ok(())
} 