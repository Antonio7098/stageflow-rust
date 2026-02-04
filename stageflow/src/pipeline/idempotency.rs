//! Idempotency enforcement utilities for WORK stages.
//!
//! Provides mechanisms to short-circuit duplicate executions based on an
//! idempotency key. Results are cached so concurrent duplicates return
//! the previously computed result instead of running the stage again.

use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::StageOutput;

/// Cached stage result with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResult {
    /// The cached stage output.
    pub output: StageOutput,
    /// Hash of the parameters used.
    pub params_hash: Option<String>,
    /// Unix timestamp when the cache entry expires.
    pub expires_at: Option<f64>,
    /// Unix timestamp when the entry was created.
    pub created_at: f64,
}

impl CachedResult {
    /// Creates a new cached result.
    #[must_use]
    pub fn new(output: StageOutput) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);

        Self {
            output,
            params_hash: None,
            expires_at: None,
            created_at: now,
        }
    }

    /// Sets the parameters hash.
    #[must_use]
    pub fn with_params_hash(mut self, hash: impl Into<String>) -> Self {
        self.params_hash = Some(hash.into());
        self
    }

    /// Sets the expiration time.
    #[must_use]
    pub fn with_ttl_seconds(mut self, ttl: f64) -> Self {
        self.expires_at = Some(self.created_at + ttl);
        self
    }

    /// Returns true if the entry has expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0);
            now >= expires_at
        } else {
            false
        }
    }
}

/// Protocol for idempotency storage backend.
#[async_trait]
pub trait IdempotencyStore: Send + Sync {
    /// Gets a cached result by key.
    async fn get(&self, key: &str) -> Option<CachedResult>;

    /// Sets a cached result.
    async fn set(&self, key: &str, entry: CachedResult, ttl_seconds: Option<f64>);

    /// Deletes a cached result.
    async fn delete(&self, key: &str);

    /// Clears all entries.
    async fn clear(&self);
}

/// In-memory idempotency store.
#[derive(Debug, Default)]
pub struct InMemoryIdempotencyStore {
    entries: Arc<Mutex<HashMap<String, CachedResult>>>,
}

impl InMemoryIdempotencyStore {
    /// Creates a new in-memory store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.lock().len()
    }

    /// Returns true if the store is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.lock().is_empty()
    }
}

#[async_trait]
impl IdempotencyStore for InMemoryIdempotencyStore {
    async fn get(&self, key: &str) -> Option<CachedResult> {
        let mut entries = self.entries.lock();
        
        if let Some(entry) = entries.get(key) {
            if entry.is_expired() {
                entries.remove(key);
                return None;
            }
            return Some(entry.clone());
        }
        
        None
    }

    async fn set(&self, key: &str, mut entry: CachedResult, ttl_seconds: Option<f64>) {
        if let Some(ttl) = ttl_seconds {
            entry = entry.with_ttl_seconds(ttl);
        }
        self.entries.lock().insert(key.to_string(), entry);
    }

    async fn delete(&self, key: &str) {
        self.entries.lock().remove(key);
    }

    async fn clear(&self) {
        self.entries.lock().clear();
    }
}

/// Configuration for idempotency enforcement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdempotencyConfig {
    /// Default TTL for cached results in seconds.
    pub default_ttl_seconds: Option<f64>,
    /// Whether to enforce parameter matching.
    pub enforce_params_match: bool,
    /// Fields to use for parameter hashing.
    pub hash_fields: Option<Vec<String>>,
}

impl Default for IdempotencyConfig {
    fn default() -> Self {
        Self {
            default_ttl_seconds: Some(3600.0), // 1 hour
            enforce_params_match: true,
            hash_fields: None,
        }
    }
}

/// Generates an idempotency key from components.
#[must_use]
pub fn generate_idempotency_key(components: &[&str]) -> String {
    let combined = components.join(":");
    let mut hasher = Sha256::new();
    hasher.update(combined.as_bytes());
    let result = hasher.finalize();
    format!("idem:{}", hex::encode(&result[..16]))
}

/// Generates a parameter hash for comparison.
#[must_use]
pub fn hash_parameters(params: &serde_json::Value, fields: Option<&[String]>) -> String {
    let to_hash = match fields {
        Some(fields) => {
            let mut filtered = serde_json::Map::new();
            if let Some(obj) = params.as_object() {
                for field in fields {
                    if let Some(value) = obj.get(field) {
                        filtered.insert(field.clone(), value.clone());
                    }
                }
            }
            serde_json::Value::Object(filtered)
        }
        None => params.clone(),
    };

    let json = serde_json::to_string(&to_hash).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..16])
}

/// Error when idempotency parameters don't match.
#[derive(Debug, Clone)]
pub struct IdempotencyParamMismatch {
    /// The idempotency key.
    pub key: String,
    /// Expected parameter hash.
    pub expected: Option<String>,
    /// Actual parameter hash.
    pub actual: Option<String>,
}

impl std::fmt::Display for IdempotencyParamMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Idempotency key '{}' parameter mismatch: expected={:?}, actual={:?}",
            self.key, self.expected, self.actual
        )
    }
}

impl std::error::Error for IdempotencyParamMismatch {}

/// Result of an idempotency check.
#[derive(Debug)]
pub enum IdempotencyCheckResult {
    /// No cached result, should execute.
    NotFound,
    /// Found cached result, return it.
    Found(CachedResult),
    /// Found but parameters don't match.
    ParamMismatch(IdempotencyParamMismatch),
}

/// Performs an idempotency check.
pub async fn check_idempotency(
    store: &dyn IdempotencyStore,
    key: &str,
    params: &serde_json::Value,
    config: &IdempotencyConfig,
) -> IdempotencyCheckResult {
    let cached = store.get(key).await;
    
    match cached {
        None => IdempotencyCheckResult::NotFound,
        Some(entry) => {
            if config.enforce_params_match {
                let current_hash = hash_parameters(params, config.hash_fields.as_deref());
                
                if let Some(ref stored_hash) = entry.params_hash {
                    if stored_hash != &current_hash {
                        return IdempotencyCheckResult::ParamMismatch(IdempotencyParamMismatch {
                            key: key.to_string(),
                            expected: Some(stored_hash.clone()),
                            actual: Some(current_hash),
                        });
                    }
                }
            }
            
            IdempotencyCheckResult::Found(entry)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cached_result_creation() {
        let output = StageOutput::ok_empty();
        let cached = CachedResult::new(output)
            .with_params_hash("abc123")
            .with_ttl_seconds(3600.0);

        assert_eq!(cached.params_hash, Some("abc123".to_string()));
        assert!(cached.expires_at.is_some());
        assert!(!cached.is_expired());
    }

    #[test]
    fn test_cached_result_expiration() {
        let output = StageOutput::ok_empty();
        let mut cached = CachedResult::new(output);
        cached.expires_at = Some(0.0); // Already expired

        assert!(cached.is_expired());
    }

    #[tokio::test]
    async fn test_in_memory_store_basic() {
        let store = InMemoryIdempotencyStore::new();
        
        assert!(store.is_empty());
        
        let entry = CachedResult::new(StageOutput::ok_empty());
        store.set("key1", entry, None).await;
        
        assert!(!store.is_empty());
        assert_eq!(store.len(), 1);
        
        let result = store.get("key1").await;
        assert!(result.is_some());
        
        store.delete("key1").await;
        assert!(store.is_empty());
    }

    #[tokio::test]
    async fn test_in_memory_store_expiration() {
        let store = InMemoryIdempotencyStore::new();
        
        let mut entry = CachedResult::new(StageOutput::ok_empty());
        entry.expires_at = Some(0.0); // Already expired
        
        store.set("expired_key", entry, None).await;
        
        // Should not return expired entry
        let result = store.get("expired_key").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_in_memory_store_clear() {
        let store = InMemoryIdempotencyStore::new();
        
        store.set("key1", CachedResult::new(StageOutput::ok_empty()), None).await;
        store.set("key2", CachedResult::new(StageOutput::ok_empty()), None).await;
        
        assert_eq!(store.len(), 2);
        
        store.clear().await;
        
        assert!(store.is_empty());
    }

    #[test]
    fn test_generate_idempotency_key() {
        let key1 = generate_idempotency_key(&["user", "123", "action"]);
        let key2 = generate_idempotency_key(&["user", "123", "action"]);
        let key3 = generate_idempotency_key(&["user", "456", "action"]);

        assert!(key1.starts_with("idem:"));
        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_hash_parameters() {
        let params1 = serde_json::json!({"a": 1, "b": 2});
        let params2 = serde_json::json!({"a": 1, "b": 2});
        let params3 = serde_json::json!({"a": 1, "b": 3});

        let hash1 = hash_parameters(&params1, None);
        let hash2 = hash_parameters(&params2, None);
        let hash3 = hash_parameters(&params3, None);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_hash_parameters_with_fields() {
        let params = serde_json::json!({"a": 1, "b": 2, "c": 3});

        let hash_all = hash_parameters(&params, None);
        let hash_ab = hash_parameters(&params, Some(&["a".to_string(), "b".to_string()]));
        let hash_a = hash_parameters(&params, Some(&["a".to_string()]));

        // Different field selections should produce different hashes
        assert_ne!(hash_all, hash_ab);
        assert_ne!(hash_ab, hash_a);
    }

    #[test]
    fn test_idempotency_config_default() {
        let config = IdempotencyConfig::default();
        assert_eq!(config.default_ttl_seconds, Some(3600.0));
        assert!(config.enforce_params_match);
    }

    #[tokio::test]
    async fn test_check_idempotency_not_found() {
        let store = InMemoryIdempotencyStore::new();
        let config = IdempotencyConfig::default();
        let params = serde_json::json!({});

        let result = check_idempotency(&store, "new_key", &params, &config).await;
        
        assert!(matches!(result, IdempotencyCheckResult::NotFound));
    }

    #[tokio::test]
    async fn test_check_idempotency_found() {
        let store = InMemoryIdempotencyStore::new();
        let config = IdempotencyConfig {
            enforce_params_match: false,
            ..Default::default()
        };
        let params = serde_json::json!({});

        let entry = CachedResult::new(StageOutput::ok_value("result", serde_json::json!(42)));
        store.set("existing_key", entry, None).await;

        let result = check_idempotency(&store, "existing_key", &params, &config).await;
        
        assert!(matches!(result, IdempotencyCheckResult::Found(_)));
    }

    #[tokio::test]
    async fn test_check_idempotency_param_mismatch() {
        let store = InMemoryIdempotencyStore::new();
        let config = IdempotencyConfig::default();
        
        let original_params = serde_json::json!({"x": 1});
        let entry = CachedResult::new(StageOutput::ok_empty())
            .with_params_hash(hash_parameters(&original_params, None));
        store.set("key", entry, None).await;

        let different_params = serde_json::json!({"x": 2});
        let result = check_idempotency(&store, "key", &different_params, &config).await;
        
        assert!(matches!(result, IdempotencyCheckResult::ParamMismatch(_)));
    }
}
