use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct NodeResponse {
    pub public_key: String,
    pub alias: String,
    pub capacity: String,
    pub first_seen: String,
}

pub struct NodeFromDb {
    pub public_key: String,
    pub alias: String,
    pub capacity: i64,
    pub first_seen: i64,
} 