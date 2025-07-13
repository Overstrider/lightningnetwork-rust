use serde::Deserialize;
use rusqlite::{params, Connection};
use std::time::Duration;
use std::env;
use log::{error, info, warn};
use reqwest::Client;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    public_key: String,
    alias: String,
    capacity: i64,
    first_seen: i64,
}

async fn fetch_nodes() -> Result<Vec<Node>, reqwest::Error> {
    info!("[Worker] Fetching nodes from the API...");
    let nodes = reqwest::get("https://mempool.space/api/v1/lightning/nodes/rankings/connectivity")
        .await?
        .json::<Vec<Node>>()
        .await?;
    Ok(nodes)
}

fn store_nodes(nodes: &[Node]) -> rusqlite::Result<(usize, usize)> {
    let db_path = env::var("DATABASE_PATH").unwrap_or("bipa.db".to_string());
    let conn = Connection::open(db_path)?;
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
    let interval_secs: u64 = env::var("FETCH_INTERVAL_SECONDS").ok().and_then(|s| s.parse().ok()).unwrap_or(10);
    let api_url = env::var("API_URL").unwrap_or("https://mempool.space/api/v1/lightning/nodes/rankings/connectivity".to_string());
    let timeout_secs: u64 = env::var("FETCH_TIMEOUT_SECONDS").ok().and_then(|s| s.parse().ok()).unwrap_or(30);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        loop {
            interval.tick().await;
            let mut attempts = 0;
            let max_attempts = 3;
            let mut backoff = 1;

            loop {
                match Client::new().get(&api_url).timeout(Duration::from_secs(timeout_secs)).send().await {
                    Ok(resp) => match resp.json::<Vec<Node>>().await {
                        Ok(nodes) => {
                            match store_nodes(&nodes) {
                                Ok((inserted, updated)) => {
                                    if inserted > 0 || updated > 0 {
                                        info!("[Worker] Done. Inserted: {}, Updated: {}.", inserted, updated);
                                    } else {
                                        info!("[Worker] All node data is already up-to-date.");
                                    }
                                    break;
                                }
                                Err(e) => {
                                    error!("[Worker] Failed to save nodes to DB: {}", e);
                                    attempts += 1;
                                    if attempts >= max_attempts {
                                        warn!("[Worker] Max retries reached for storing nodes.");
                                        break;
                                    }
                                    tokio::time::sleep(Duration::from_secs(backoff)).await;
                                    backoff *= 2;
                                }
                            }
                        }
                        Err(e) => {
                            error!("[Worker] Failed to parse nodes from API: {}", e);
                            attempts += 1;
                            if attempts >= max_attempts {
                                warn!("[Worker] Max retries reached for parsing.");
                                break;
                            }
                            tokio::time::sleep(Duration::from_secs(backoff)).await;
                            backoff *= 2;
                        }
                    },
                    Err(e) => {
                        error!("[Worker] Failed to fetch nodes from API: {}", e);
                        attempts += 1;
                        if attempts >= max_attempts {
                            warn!("[Worker] Max retries reached for fetch.");
                            break;
                        }
                        tokio::time::sleep(Duration::from_secs(backoff)).await;
                        backoff *= 2;
                    }
                }
            }
        }
    });
} 