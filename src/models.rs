use serde::Serialize;

// Just a home for the data structures we use in the app.

/// How a node is represented in our API response (GET /nodes).
#[derive(Serialize, Clone)]
pub struct NodeResponse {
    pub public_key: String,
    pub alias: String,
    pub capacity: String,
    pub first_seen: String,
}

/// How a node is represented when we read it from the database,
/// before formatting the fields for the API response.
pub struct NodeFromDb {
    pub public_key: String,
    pub alias: String,
    pub capacity: i64,
    pub first_seen: i64,
} 