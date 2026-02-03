//! Memory helpers for conversation history.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A memory entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub session_id: Uuid,
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl MemoryEntry {
    /// Converts to a dictionary.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("id".to_string(), serde_json::json!(self.id));
        map.insert("session_id".to_string(), serde_json::json!(self.session_id.to_string()));
        map.insert("role".to_string(), serde_json::json!(self.role));
        map.insert("content".to_string(), serde_json::json!(self.content));
        map.insert("timestamp".to_string(), serde_json::json!(self.timestamp.to_rfc3339()));
        map
    }
}

/// Memory configuration.
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    pub max_entries: usize,
    pub max_tokens: usize,
    pub include_system: bool,
    pub recency_window_seconds: u64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self { max_entries: 20, max_tokens: 4000, include_system: true, recency_window_seconds: 0 }
    }
}

/// In-memory store for memory entries.
#[derive(Default)]
pub struct InMemoryStore {
    entries: parking_lot::RwLock<HashMap<Uuid, Vec<MemoryEntry>>>,
}

impl InMemoryStore {
    /// Creates a new store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores an entry.
    pub fn store(&self, entry: MemoryEntry) {
        self.entries.write().entry(entry.session_id).or_default().push(entry);
    }

    /// Fetches entries for a session.
    #[must_use]
    pub fn fetch(&self, session_id: Uuid, config: &MemoryConfig) -> Vec<MemoryEntry> {
        self.entries.read().get(&session_id).cloned().unwrap_or_default()
            .into_iter()
            .filter(|e| config.include_system || e.role != "system")
            .rev()
            .take(config.max_entries)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }
}

/// Memory fetch stage.
pub struct MemoryFetchStage {
    store: std::sync::Arc<InMemoryStore>,
    config: MemoryConfig,
}

impl MemoryFetchStage {
    /// Creates a new fetch stage.
    #[must_use]
    pub fn new(store: std::sync::Arc<InMemoryStore>, config: MemoryConfig) -> Self {
        Self { store, config }
    }
}
