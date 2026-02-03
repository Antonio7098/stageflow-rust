//! Undo metadata and store.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Metadata for undoing a tool action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoMetadata {
    /// The action ID this undo is for.
    pub action_id: Uuid,
    /// The tool name.
    pub tool_name: String,
    /// Data needed to perform the undo.
    pub undo_data: serde_json::Value,
    /// When this was created (ISO 8601).
    pub created_at: String,
}

impl UndoMetadata {
    /// Creates new undo metadata.
    #[must_use]
    pub fn new(action_id: Uuid, tool_name: impl Into<String>, undo_data: serde_json::Value) -> Self {
        Self {
            action_id,
            tool_name: tool_name.into(),
            undo_data,
            created_at: crate::utils::iso_timestamp(),
        }
    }

    /// Creates from a dictionary.
    pub fn from_dict(dict: &HashMap<String, serde_json::Value>) -> Option<Self> {
        let action_id = dict
            .get("action_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())?;

        let tool_name = dict.get("tool_name").and_then(|v| v.as_str())?.to_string();

        let undo_data = dict.get("undo_data").cloned().unwrap_or(serde_json::json!({}));

        let created_at = dict
            .get("created_at")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(crate::utils::iso_timestamp);

        Some(Self {
            action_id,
            tool_name,
            undo_data,
            created_at,
        })
    }

    /// Converts to a dictionary.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("action_id".to_string(), serde_json::json!(self.action_id.to_string()));
        map.insert("tool_name".to_string(), serde_json::json!(self.tool_name));
        map.insert("undo_data".to_string(), self.undo_data.clone());
        map.insert("created_at".to_string(), serde_json::json!(self.created_at));
        map
    }
}

/// Entry in the undo store with TTL.
struct UndoEntry {
    metadata: UndoMetadata,
    expires_at: Instant,
}

/// Store for undo metadata with TTL.
pub struct UndoStore {
    /// TTL for entries.
    ttl: Duration,
    /// Stored entries.
    entries: RwLock<HashMap<Uuid, UndoEntry>>,
}

impl UndoStore {
    /// Creates a new undo store.
    #[must_use]
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            entries: RwLock::new(HashMap::new()),
        }
    }

    /// Stores undo metadata.
    pub fn store(&self, metadata: UndoMetadata) {
        let entry = UndoEntry {
            metadata: metadata.clone(),
            expires_at: Instant::now() + self.ttl,
        };
        self.entries.write().insert(metadata.action_id, entry);
    }

    /// Gets undo metadata for an action.
    ///
    /// Returns None if not found or expired.
    #[must_use]
    pub fn get(&self, action_id: Uuid) -> Option<UndoMetadata> {
        let mut entries = self.entries.write();

        if let Some(entry) = entries.get(&action_id) {
            if entry.expires_at > Instant::now() {
                return Some(entry.metadata.clone());
            } else {
                // Expired, remove it
                entries.remove(&action_id);
            }
        }
        None
    }

    /// Removes undo metadata.
    pub fn remove(&self, action_id: Uuid) -> bool {
        self.entries.write().remove(&action_id).is_some()
    }

    /// Clears all entries.
    pub fn clear(&self) {
        self.entries.write().clear();
    }

    /// Returns the number of entries (including potentially expired ones).
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Returns true if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }

    /// Cleans up expired entries.
    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        self.entries.write().retain(|_, entry| entry.expires_at > now);
    }
}

impl Default for UndoStore {
    fn default() -> Self {
        Self::new(Duration::from_secs(3600)) // 1 hour default
    }
}

impl std::fmt::Debug for UndoStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UndoStore")
            .field("ttl", &self.ttl)
            .field("entries", &self.len())
            .finish()
    }
}

// Global singleton
static GLOBAL_STORE: RwLock<Option<Arc<UndoStore>>> = RwLock::new(None);

/// Gets the global undo store.
pub fn get_undo_store() -> Arc<UndoStore> {
    let read = GLOBAL_STORE.read();
    if let Some(ref store) = *read {
        return store.clone();
    }
    drop(read);

    let mut write = GLOBAL_STORE.write();
    if write.is_none() {
        *write = Some(Arc::new(UndoStore::default()));
    }
    write.as_ref().unwrap().clone()
}

/// Sets the global undo store.
pub fn set_undo_store(store: Arc<UndoStore>) {
    *GLOBAL_STORE.write() = Some(store);
}

/// Clears the global undo store.
pub fn clear_undo_store() {
    *GLOBAL_STORE.write() = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undo_metadata_creation() {
        let action_id = Uuid::new_v4();
        let metadata = UndoMetadata::new(action_id, "my_tool", serde_json::json!({"key": "value"}));

        assert_eq!(metadata.action_id, action_id);
        assert_eq!(metadata.tool_name, "my_tool");
    }

    #[test]
    fn test_undo_metadata_roundtrip() {
        let action_id = Uuid::new_v4();
        let metadata = UndoMetadata::new(action_id, "tool", serde_json::json!({}));

        let dict = metadata.to_dict();
        let restored = UndoMetadata::from_dict(&dict).unwrap();

        assert_eq!(metadata.action_id, restored.action_id);
        assert_eq!(metadata.tool_name, restored.tool_name);
    }

    #[test]
    fn test_undo_store_basic() {
        let store = UndoStore::new(Duration::from_secs(60));
        let action_id = Uuid::new_v4();

        let metadata = UndoMetadata::new(action_id, "tool", serde_json::json!({}));
        store.store(metadata);

        assert!(store.get(action_id).is_some());
        assert!(store.get(Uuid::new_v4()).is_none());
    }

    #[test]
    fn test_undo_store_expiry() {
        let store = UndoStore::new(Duration::from_millis(1));
        let action_id = Uuid::new_v4();

        let metadata = UndoMetadata::new(action_id, "tool", serde_json::json!({}));
        store.store(metadata);

        // Wait for expiry
        std::thread::sleep(Duration::from_millis(10));

        assert!(store.get(action_id).is_none());
    }

    #[test]
    fn test_undo_store_remove() {
        let store = UndoStore::new(Duration::from_secs(60));
        let action_id = Uuid::new_v4();

        let metadata = UndoMetadata::new(action_id, "tool", serde_json::json!({}));
        store.store(metadata);

        assert!(store.remove(action_id));
        assert!(store.get(action_id).is_none());
    }
}
