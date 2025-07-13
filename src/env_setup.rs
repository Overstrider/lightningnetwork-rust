use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn setup_env() -> std::io::Result<()> {
    let env_path = Path::new(".env");
    if !env_path.exists() {
        let mut file = File::create(env_path)?;
        let content = r#"
DATABASE_PATH="bipa.db"
API_URL="https://mempool.space/api/v1/lightning/nodes/rankings/connectivity"
FETCH_INTERVAL_SECONDS=1
FETCH_TIMEOUT_SECONDS=30
SERVER_PORT=8080
CACHE_TTL_SECONDS=10
RUST_LOG=debug
"#;
        file.write_all(content.as_bytes())?;
        println!("[Env] Created .env file with default configurations.");
    }
    Ok(())
} 