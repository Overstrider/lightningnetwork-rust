use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use rusqlite::Connection;
use std::env;
use log::{error, info};
use dotenvy::dotenv;
use moka::future::Cache;
mod db;
mod worker;
mod formatters;
mod env_setup;
mod models;
use models::{NodeResponse, NodeFromDb};

/// Handler for the GET /nodes endpoint.
///
/// It serves node data, trying the cache first. If the cache is empty,
/// it falls back to querying the database. The database itself is updated
/// by a background worker, so this function is read-only.
#[get("/nodes")]
async fn get_nodes(cache: web::Data<Cache<String, Vec<NodeResponse>>>) -> impl Responder {
    let db_path = env::var("DATABASE_PATH").unwrap_or("bipa.db".to_string());
    let cache_key = "nodes".to_string();

    // Try to get the response from the cache.
    if let Some(cached_nodes) = cache.get(&cache_key).await {
        info!("[API] Cache hit for /nodes");
        return HttpResponse::Ok().json(cached_nodes);
    }
    info!("[API] Cache miss for /nodes");

    // If cache is empty, query the database.
    // We run this in a blocking thread to avoid holding up the server.
    let result = web::block(move || -> Result<Vec<NodeResponse>, rusqlite::Error> {
        let conn = Connection::open(&db_path)?;
        let mut stmt = conn.prepare("SELECT public_key, alias, capacity, first_seen FROM nodes ORDER BY capacity DESC")?;
        
        let node_iter = stmt.query_map([], |row| {
            Ok(NodeFromDb {
                public_key: row.get(0)?,
                alias: row.get(1)?,
                capacity: row.get(2)?,
                first_seen: row.get(3)?,
            })
        })?;

        let mut nodes = Vec::new();
        for node_result in node_iter {
            let node_db = node_result?;
            nodes.push(NodeResponse {
                public_key: node_db.public_key,
                alias: node_db.alias,
                capacity: formatters::format_capacity(node_db.capacity),
                first_seen: formatters::format_timestamp(node_db.first_seen),
            });
        }
        Ok(nodes)
    })
    .await;

    match result {
        Ok(Ok(nodes)) => {
            // Put the result in the cache for next time.
            cache.insert(cache_key.clone(), nodes.clone()).await;
            HttpResponse::Ok().json(nodes)
        }
        Ok(Err(e)) => {
            error!("DB error: {}", e);
            HttpResponse::InternalServerError().body("Error fetching nodes from database")
        }
        Err(e) => {
            error!("Task error: {}", e);
            HttpResponse::InternalServerError().body("Internal server error")
        }
    }
}

/// This is where the app starts.
///
/// It sets up everything: .env, logger, database, the background worker,
/// the cache, and finally, the web server.
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Create a default .env file if needed, then load it.
    env_setup::setup_env()?;
    dotenv().ok();
    env_logger::init();

    // Set up the database. The app won't start if this fails.
    let db_path = env::var("DATABASE_PATH").unwrap_or("bipa.db".to_string());
    if let Err(e) = db::initialize_database(&db_path) {
        error!("Failed to start database: {}", e);
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Database initialization failed"));
    }
    info!("[Main] Database is ready.");

    // Start the background worker.
    worker::spawn_worker();
    info!("[Main] Background worker started.");

    // Set up the cache. TTL is configurable via .env.
    let port: u16 = env::var("SERVER_PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(8080);
    let ttl_secs: u64 = env::var("CACHE_TTL_SECONDS").ok().and_then(|s| s.parse().ok()).unwrap_or(10);
    let cache: Cache<String, Vec<NodeResponse>> = Cache::builder()
        .time_to_live(std::time::Duration::from_secs(ttl_secs))
        .build();

    // Start the HTTP server and share the cache with all threads.
    info!("Starting server on http://0.0.0.0:{}", port);
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(cache.clone()))
            .service(get_nodes)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}

