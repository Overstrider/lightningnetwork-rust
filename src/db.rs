use rusqlite::{Connection, Result, OpenFlags};
use chrono::DateTime;
use log::{error, info};

fn needs_migration(conn: &Connection) -> Result<bool> {
    let mut stmt = conn.prepare("PRAGMA table_info(nodes)")?;
    let column_types: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(1)?, row.get(2)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    // Se a coluna 'first_seen' for do tipo TEXT, a migração é necessária.
    if let Some((_, col_type)) = column_types.iter().find(|(name, _)| name == "first_seen") {
        if col_type.eq_ignore_ascii_case("TEXT") {
            return Ok(true);
        }
    }

    Ok(false)
}

fn run_migration(conn: &mut Connection) -> Result<()> {
    info!("[DB] Detected old schema, running migration for 'nodes' table...");
    
    let tx = conn.transaction()?;

    tx.execute("ALTER TABLE nodes RENAME TO nodes_old_migration_temp", [])?;

    tx.execute(
        "CREATE TABLE nodes (
            public_key    TEXT PRIMARY KEY,
            alias         TEXT NOT NULL,
            capacity      INTEGER NOT NULL,
            first_seen    INTEGER NOT NULL
        )",
        [],
    )?;

    // Migra os dados da tabela antiga para a nova.
    {
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
            // Converte a string de data antiga para um timestamp.
            let first_seen_ts = DateTime::parse_from_rfc3339(&old_node.first_seen)
                                    .map(|dt| dt.timestamp())
                                    .unwrap_or_else(|e| {
                                        error!("Failed to parse first_seen '{}': {}", old_node.first_seen, e);
                                        0
                                    });

            tx.execute(
                "INSERT OR IGNORE INTO nodes (public_key, alias, capacity, first_seen) VALUES (?1, ?2, ?3, ?4)",
                (&old_node.public_key, &old_node.alias, &old_node.capacity, &first_seen_ts),
            )?;
        }
    }
    
    tx.execute("DROP TABLE nodes_old_migration_temp", [])?;

    tx.commit()?;
    info!("[DB] Migration completed successfully.");
    Ok(())
}


pub fn initialize_database(db_path: &str) -> Result<()> {
    let mut conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_NO_MUTEX).map(|c| {
        c.busy_timeout(std::time::Duration::from_secs(5));
        c
    })?;

    let table_exists: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type='table' AND name='nodes')",
        [],
        |row| row.get(0),
    )?;

    if !table_exists {
        // Se a tabela não existe, cria com o esquema novo.
        println!("[DB] 'nodes' table not found, creating new one.");
        conn.execute(
            "CREATE TABLE nodes (
                public_key    TEXT PRIMARY KEY,
                alias         TEXT NOT NULL,
                capacity      INTEGER NOT NULL,
                first_seen    INTEGER NOT NULL
            )",
            [],
        )?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_capacity ON nodes(capacity DESC)", [])?;
    } else {
        // Se a tabela existe, verifica se precisa de migração.
        if needs_migration(&conn)? {
            run_migration(&mut conn)?;
        }
    }

    Ok(())
} 