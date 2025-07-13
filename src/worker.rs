use serde::Deserialize;
use rusqlite::{params, Connection};
use std::time::Duration;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    public_key: String,
    alias: String,
    capacity: i64,
    first_seen: i64,
}

async fn fetch_nodes() -> Result<Vec<Node>, reqwest::Error> {
    println!("[Worker] Fetching nodes from the API...");
    let nodes = reqwest::get("https://mempool.space/api/v1/lightning/nodes/rankings/connectivity")
        .await?
        .json::<Vec<Node>>()
        .await?;
    Ok(nodes)
}

fn store_nodes(nodes: &[Node]) -> rusqlite::Result<(usize, usize)> {
    let conn = Connection::open("bipa.db")?;
    let tx = conn.unchecked_transaction()?;

    let mut inserted_count = 0;
    let mut updated_count = 0;

    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO nodes (public_key, alias, capacity, first_seen) VALUES (?1, ?2, ?3, ?4)",
        )?;
        for node in nodes {
            let changed = stmt.execute(params![
                node.public_key,
                node.alias,
                node.capacity,
                node.first_seen
            ])?;
            inserted_count += changed;
        }
    }

    {
        let mut stmt = tx.prepare_cached(
            "UPDATE nodes SET alias = ?2, capacity = ?3 WHERE public_key = ?1 AND (alias != ?2 OR capacity != ?3)",
        )?;
        for node in nodes {
            let changed = stmt.execute(params![node.public_key, node.alias, node.capacity])?;
            updated_count += changed;
        }
    }

    tx.commit()?;
    Ok((inserted_count, updated_count))
}

pub fn spawn_worker() {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            match fetch_nodes().await {
                Ok(nodes) => {
                    match store_nodes(&nodes) {
                        Ok((inserted, updated)) => {
                            if inserted > 0 || updated > 0 {
                                println!("[Worker] Done. Inserted: {}, Updated: {}.", inserted, updated);
                            } else {
                                println!("[Worker] All node data is already up-to-date.");
                            }
                        }
                        Err(e) => eprintln!("[Worker] Failed to save nodes to DB: {}", e),
                    }
                }
                Err(e) => eprintln!("[Worker] Failed to fetch nodes from API: {}", e),
            }
        }
    });
} 