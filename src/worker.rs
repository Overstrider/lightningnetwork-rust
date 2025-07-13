use serde::Deserialize;
use rusqlite::{params, Connection};
use std::time::Duration;
use std::env;
use log::{error, info, warn};
use reqwest::Client;

// This module is the background worker. It's job is to fetch node data
// from the API and save it to our local database on a timer.

/// The node data we get from the Mempool API.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    public_key: String,
    alias: String,
    capacity: i64,
    first_seen: i64,
}

/// Grabs the latest node data from the Mempool API.
async fn fetch_nodes(api_url: &str, client: &Client) -> Result<Vec<Node>, reqwest::Error> {
    info!("[Worker] Fetching nodes from API...");
    let nodes = client
        .get(api_url)
        .send()
        .await?
        .json::<Vec<Node>>()
        .await?;
    Ok(nodes)
}

/// Saves the list of nodes into the database.
///
/// It does two things in one transaction:
/// 1. `INSERT OR IGNORE`: Adds any new nodes.
/// 2. `UPDATE`: Updates info for existing nodes if it changed.
///
/// This is way more efficient than checking each node one by one.
fn store_nodes(nodes: &[Node]) -> rusqlite::Result<(usize, usize)> {
    let db_path = env::var("DATABASE_PATH").unwrap_or("nodes.db".to_string());
    let conn = rusqlite::Connection::open(db_path)?;
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

    // Commit the transaction to make the changes permanent.
    tx.commit()?;
    Ok((inserted_count, updated_count))
}

/// Kicks off the background worker task.
///
/// This function spawns a Tokio task that runs in a loop.
/// It fetches data on a timer and will retry a few times with a delay
/// if the API or database fails, so it's pretty resilient.
pub fn spawn_worker() {
    let interval_secs: u64 = env::var("FETCH_INTERVAL_SECONDS").ok().and_then(|s| s.parse().ok()).unwrap_or(10);
    let api_url = env::var("API_URL").unwrap_or("https://mempool.space/api/v1/lightning/nodes/rankings/connectivity".to_string());
    let timeout_secs: u64 = env::var("FETCH_TIMEOUT_SECONDS").ok().and_then(|s| s.parse().ok()).unwrap_or(30);
    
    let client = Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .expect("Failed to build reqwest client");

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        loop {
            // Wait for the next tick.
            interval.tick().await;

            // Simple retry loop.
            let mut attempts = 0;
            let max_attempts = 3;
            let mut backoff = 1;

            loop {
                match fetch_nodes(&api_url, &client).await {
                    Ok(nodes) => {
                        // Got the nodes, now try to save them.
                        match store_nodes(&nodes) {
                            Ok((inserted, updated)) => {
                                if inserted > 0 || updated > 0 {
                                    info!("[Worker] DB updated. Inserted: {}, Updated: {}.", inserted, updated);
                                }
                                break; // All good, break the retry loop.
                            }
                            Err(e) => error!("[Worker] Failed to save nodes to DB: {}", e),
                        }
                    }
                    Err(e) => error!("[Worker] Failed to fetch nodes from API: {}", e),
                }

                // If we're here, something failed. Time to retry.
                attempts += 1;
                if attempts >= max_attempts {
                    warn!("[Worker] Max retries reached. Will try again later.");
                    break;
                }
                
                info!("[Worker] Retrying in {}s...", backoff);
                tokio::time::sleep(Duration::from_secs(backoff)).await;
                backoff *= 2; // Double the wait time for next retry.
            }
        }
    });
} 