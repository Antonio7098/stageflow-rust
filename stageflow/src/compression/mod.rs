//! Compression utilities for context delta encoding.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metrics about a compression operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionMetrics {
    /// Original size in bytes.
    pub original_bytes: usize,
    /// Delta size in bytes.
    pub delta_bytes: usize,
    /// Reduction in bytes.
    pub reduction_bytes: usize,
    /// Compression ratio (delta/original).
    pub ratio: f64,
}

impl CompressionMetrics {
    /// Creates new metrics.
    #[must_use]
    pub fn new(original_bytes: usize, delta_bytes: usize) -> Self {
        let reduction_bytes = original_bytes.saturating_sub(delta_bytes);
        let ratio = if original_bytes == 0 {
            1.0
        } else {
            delta_bytes as f64 / original_bytes as f64
        };

        Self {
            original_bytes,
            delta_bytes,
            reduction_bytes,
            ratio,
        }
    }
}

/// Computes a shallow delta between two dictionaries.
///
/// Returns a delta object with:
/// - `set`: keys that are new or changed
/// - `remove`: keys that were removed
#[must_use]
pub fn compute_delta(
    base: &HashMap<String, serde_json::Value>,
    current: &HashMap<String, serde_json::Value>,
) -> HashMap<String, serde_json::Value> {
    let mut delta = HashMap::new();

    // Find set operations (new or changed)
    let mut set_ops: HashMap<String, serde_json::Value> = HashMap::new();
    for (key, value) in current {
        match base.get(key) {
            None => {
                set_ops.insert(key.clone(), value.clone());
            }
            Some(base_value) if base_value != value => {
                set_ops.insert(key.clone(), value.clone());
            }
            _ => {}
        }
    }

    // Find remove operations
    let remove_ops: Vec<String> = base
        .keys()
        .filter(|k| !current.contains_key(*k))
        .cloned()
        .collect();

    if !set_ops.is_empty() {
        let set_map: serde_json::Map<String, serde_json::Value> = set_ops.into_iter().collect();
        delta.insert("set".to_string(), serde_json::Value::Object(set_map));
    }

    if !remove_ops.is_empty() {
        delta.insert(
            "remove".to_string(),
            serde_json::Value::Array(remove_ops.into_iter().map(serde_json::Value::String).collect()),
        );
    }

    delta
}

/// Applies a delta to a base dictionary.
#[must_use]
pub fn apply_delta(
    base: &HashMap<String, serde_json::Value>,
    delta: &HashMap<String, serde_json::Value>,
) -> HashMap<String, serde_json::Value> {
    let mut result = base.clone();

    // Apply removes
    if let Some(serde_json::Value::Array(removes)) = delta.get("remove") {
        for remove in removes {
            if let Some(key) = remove.as_str() {
                result.remove(key);
            }
        }
    }

    // Apply sets
    if let Some(serde_json::Value::Object(sets)) = delta.get("set") {
        for (key, value) in sets {
            result.insert(key.clone(), value.clone());
        }
    }

    result
}

/// Compresses current state relative to base and returns delta with metrics.
pub fn compress(
    base: &HashMap<String, serde_json::Value>,
    current: &HashMap<String, serde_json::Value>,
) -> (HashMap<String, serde_json::Value>, CompressionMetrics) {
    let delta = compute_delta(base, current);

    let original_bytes = json_safe_bytes(current);
    let delta_bytes = json_safe_bytes(&delta);

    let metrics = CompressionMetrics::new(original_bytes, delta_bytes);

    (delta, metrics)
}

fn json_safe_bytes(data: &HashMap<String, serde_json::Value>) -> usize {
    serde_json::to_string(&make_json_safe(data))
        .map(|s| s.len())
        .unwrap_or(0)
}

fn make_json_safe(data: &HashMap<String, serde_json::Value>) -> serde_json::Value {
    // Already JSON-safe, just convert
    let map: serde_json::Map<String, serde_json::Value> = data
        .iter()
        .map(|(k, v)| (k.clone(), make_value_json_safe(v)))
        .collect();
    serde_json::Value::Object(map)
}

fn make_value_json_safe(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(make_value_json_safe).collect())
        }
        serde_json::Value::Object(obj) => {
            let safe_obj: serde_json::Map<String, serde_json::Value> = obj
                .iter()
                .map(|(k, v)| (k.clone(), make_value_json_safe(v)))
                .collect();
            serde_json::Value::Object(safe_obj)
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_delta_new_key() {
        let base = HashMap::new();
        let mut current = HashMap::new();
        current.insert("key".to_string(), serde_json::json!("value"));

        let delta = compute_delta(&base, &current);

        assert!(delta.contains_key("set"));
    }

    #[test]
    fn test_compute_delta_changed_key() {
        let mut base = HashMap::new();
        base.insert("key".to_string(), serde_json::json!("old"));

        let mut current = HashMap::new();
        current.insert("key".to_string(), serde_json::json!("new"));

        let delta = compute_delta(&base, &current);

        assert!(delta.contains_key("set"));
    }

    #[test]
    fn test_compute_delta_removed_key() {
        let mut base = HashMap::new();
        base.insert("key".to_string(), serde_json::json!("value"));

        let current = HashMap::new();

        let delta = compute_delta(&base, &current);

        assert!(delta.contains_key("remove"));
    }

    #[test]
    fn test_compute_delta_no_changes() {
        let mut base = HashMap::new();
        base.insert("key".to_string(), serde_json::json!("value"));

        let delta = compute_delta(&base, &base);

        assert!(delta.is_empty());
    }

    #[test]
    fn test_apply_delta_roundtrip() {
        let mut base = HashMap::new();
        base.insert("a".to_string(), serde_json::json!(1));
        base.insert("b".to_string(), serde_json::json!(2));

        let mut current = HashMap::new();
        current.insert("a".to_string(), serde_json::json!(10)); // Changed
        current.insert("c".to_string(), serde_json::json!(3)); // New
        // "b" removed

        let delta = compute_delta(&base, &current);
        let result = apply_delta(&base, &delta);

        assert_eq!(result, current);
    }

    #[test]
    fn test_compression_metrics() {
        let base = HashMap::new();
        let mut current = HashMap::new();
        current.insert("key".to_string(), serde_json::json!("value"));

        let (delta, metrics) = compress(&base, &current);

        assert!(metrics.original_bytes > 0);
        assert!(metrics.delta_bytes > 0);
    }
}
