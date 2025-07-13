use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use rusqlite::Connection;
use std::env;
use log::{error, info};
use dotenvy::dotenv;
use moka::future::Cache;
use std::sync::Arc;
mod db;
mod worker;
mod formatters;
mod env_setup;
mod models;
use models::{NodeResponse, NodeFromDb};

#[get("/nodes")]
async fn get_nodes(cache: web::Data<Cache<String, Vec<NodeResponse>>>) -> impl Responder {
    let db_path = env::var("DATABASE_PATH").unwrap_or("bipa.db".to_string());
    let cache_key = "nodes".to_string();
    if let Some(cached_nodes) = cache.get(&cache_key).await {
        return HttpResponse::Ok().json(cached_nodes);
    }

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
            cache.insert(cache_key.clone(), nodes.clone()).await;
            HttpResponse::Ok().json(nodes)
        }
        Ok(Err(e)) => {
            error!("Error fetching nodes from database: {}", e);
            HttpResponse::InternalServerError().body("Error fetching nodes from database")
        }
        Err(e) => {
            error!("Blocking error: {}", e);
            HttpResponse::InternalServerError().body("Internal server error")
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_setup::setup_env()?;
    dotenv().ok(); // Carrega .env se existir
    env_logger::init();
    let db_path_arc = Arc::new(env::var("DATABASE_PATH").unwrap_or("bipa.db".to_string()));
    if let Err(e) = db::initialize_database(&db_path_arc) {
        error!("Failed to initialize the database: {}", e);
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Database initialization failed"));
    }
    info!("[Main] Database initialized");

    worker::spawn_worker();
    info!("[Main] Worker thread started.");

    let port: u16 = env::var("SERVER_PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(8080);
    let ttl_secs: u64 = env::var("CACHE_TTL_SECONDS").ok().and_then(|s| s.parse().ok()).unwrap_or(10);
    let cache: Cache<String, Vec<NodeResponse>> = Cache::builder()
        .time_to_live(std::time::Duration::from_secs(ttl_secs))
        .build();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(cache.clone()))
            .service(get_nodes)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}

