// Hermes Memory — Three-layer (session, project, user) with SQLite + FTS5 + embeddings
// Every interaction is stored and retrievable via semantic search.

pub mod session;
pub mod project;
pub mod user;
pub mod embed;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub layer: MemoryLayer,
    pub key: String,
    pub value: String,
    pub embedding: Option<Vec<f32>>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryLayer {
    Session,
    Project,
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub entries: Vec<MemoryEntry>,
    pub relevance: Vec<f64>,
}
