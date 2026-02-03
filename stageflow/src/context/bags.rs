//! Thread-safe context and output bags.

use crate::errors::{DataConflictError, OutputConflictError};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// A thread-safe bag for storing context data.
///
/// Writing to an existing key raises a `DataConflictError`.
#[derive(Debug, Default)]
pub struct ContextBag {
    data: RwLock<HashMap<String, serde_json::Value>>,
}

impl ContextBag {
    /// Creates a new empty context bag.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a context bag from existing data.
    #[must_use]
    pub fn from_data(data: HashMap<String, serde_json::Value>) -> Self {
        Self {
            data: RwLock::new(data),
        }
    }

    /// Gets a value from the bag.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<serde_json::Value> {
        self.data.read().get(key).cloned()
    }

    /// Checks if a key exists.
    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.data.read().contains_key(key)
    }

    /// Sets a value in the bag.
    ///
    /// # Errors
    ///
    /// Returns `DataConflictError` if the key already exists.
    pub fn set(&self, key: impl Into<String>, value: serde_json::Value) -> Result<(), DataConflictError> {
        let key = key.into();
        let mut data = self.data.write();

        if data.contains_key(&key) {
            return Err(DataConflictError::new(&key));
        }

        data.insert(key, value);
        Ok(())
    }

    /// Sets a value, allowing overwrites.
    pub fn set_force(&self, key: impl Into<String>, value: serde_json::Value) {
        self.data.write().insert(key.into(), value);
    }

    /// Returns a copy of all data.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        self.data.read().clone()
    }

    /// Returns the number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.read().len()
    }

    /// Returns true if the bag is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.read().is_empty()
    }

    /// Returns all keys.
    #[must_use]
    pub fn keys(&self) -> Vec<String> {
        self.data.read().keys().cloned().collect()
    }
}

impl Clone for ContextBag {
    fn clone(&self) -> Self {
        Self {
            data: RwLock::new(self.data.read().clone()),
        }
    }
}

/// Per-stage output entry with attempt tracking.
#[derive(Debug, Clone)]
pub struct StageOutputEntry {
    /// The stage output data.
    pub data: HashMap<String, serde_json::Value>,
    /// The attempt number (1-indexed).
    pub attempt: u32,
    /// Whether this is a final output.
    pub is_final: bool,
}

/// A thread-safe bag for storing per-stage outputs.
///
/// Supports retry semantics with attempt tracking.
#[derive(Debug, Default)]
pub struct OutputBag {
    outputs: RwLock<HashMap<String, StageOutputEntry>>,
}

impl OutputBag {
    /// Creates a new empty output bag.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets output for a stage.
    #[must_use]
    pub fn get(&self, stage: &str) -> Option<HashMap<String, serde_json::Value>> {
        self.outputs.read().get(stage).map(|e| e.data.clone())
    }

    /// Gets the full output entry for a stage.
    #[must_use]
    pub fn get_entry(&self, stage: &str) -> Option<StageOutputEntry> {
        self.outputs.read().get(stage).cloned()
    }

    /// Checks if output exists for a stage.
    #[must_use]
    pub fn contains(&self, stage: &str) -> bool {
        self.outputs.read().contains_key(stage)
    }

    /// Sets output for a stage.
    ///
    /// # Errors
    ///
    /// Returns `OutputConflictError` if the stage already has a final output.
    pub fn set(
        &self,
        stage: impl Into<String>,
        data: HashMap<String, serde_json::Value>,
        attempt: u32,
        is_final: bool,
    ) -> Result<(), OutputConflictError> {
        let stage = stage.into();
        let mut outputs = self.outputs.write();

        if let Some(existing) = outputs.get(&stage) {
            if existing.is_final {
                return Err(OutputConflictError::new(
                    &stage,
                    "Stage already has a final output",
                ));
            }
        }

        outputs.insert(
            stage,
            StageOutputEntry {
                data,
                attempt,
                is_final,
            },
        );

        Ok(())
    }

    /// Sets output, allowing overwrites (for guard-retry scenarios).
    pub fn set_force(
        &self,
        stage: impl Into<String>,
        data: HashMap<String, serde_json::Value>,
        attempt: u32,
        is_final: bool,
    ) {
        self.outputs.write().insert(
            stage.into(),
            StageOutputEntry {
                data,
                attempt,
                is_final,
            },
        );
    }

    /// Returns a copy of all outputs.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, HashMap<String, serde_json::Value>> {
        self.outputs
            .read()
            .iter()
            .map(|(k, v)| (k.clone(), v.data.clone()))
            .collect()
    }

    /// Returns the number of stages with outputs.
    #[must_use]
    pub fn len(&self) -> usize {
        self.outputs.read().len()
    }

    /// Returns true if no outputs have been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.outputs.read().is_empty()
    }

    /// Returns all stage names with outputs.
    #[must_use]
    pub fn stages(&self) -> Vec<String> {
        self.outputs.read().keys().cloned().collect()
    }
}

impl Clone for OutputBag {
    fn clone(&self) -> Self {
        Self {
            outputs: RwLock::new(self.outputs.read().clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_bag_set_and_get() {
        let bag = ContextBag::new();
        bag.set("key", serde_json::json!("value")).unwrap();

        assert_eq!(bag.get("key"), Some(serde_json::json!("value")));
        assert!(bag.contains_key("key"));
        assert!(!bag.contains_key("other"));
    }

    #[test]
    fn test_context_bag_conflict() {
        let bag = ContextBag::new();
        bag.set("key", serde_json::json!(1)).unwrap();

        let result = bag.set("key", serde_json::json!(2));
        assert!(result.is_err());
    }

    #[test]
    fn test_context_bag_force() {
        let bag = ContextBag::new();
        bag.set("key", serde_json::json!(1)).unwrap();
        bag.set_force("key", serde_json::json!(2));

        assert_eq!(bag.get("key"), Some(serde_json::json!(2)));
    }

    #[test]
    fn test_context_bag_to_dict() {
        let bag = ContextBag::new();
        bag.set("a", serde_json::json!(1)).unwrap();
        bag.set("b", serde_json::json!(2)).unwrap();

        let dict = bag.to_dict();
        assert_eq!(dict.len(), 2);
    }

    #[test]
    fn test_output_bag_set_and_get() {
        let bag = OutputBag::new();
        let mut data = HashMap::new();
        data.insert("result".to_string(), serde_json::json!("ok"));

        bag.set("stage1", data.clone(), 1, true).unwrap();

        assert_eq!(bag.get("stage1"), Some(data));
        assert!(bag.contains("stage1"));
    }

    #[test]
    fn test_output_bag_conflict_on_final() {
        let bag = OutputBag::new();
        let data = HashMap::new();

        bag.set("stage1", data.clone(), 1, true).unwrap();

        let result = bag.set("stage1", data, 2, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_output_bag_overwrite_non_final() {
        let bag = OutputBag::new();
        let mut data1 = HashMap::new();
        data1.insert("x".to_string(), serde_json::json!(1));

        let mut data2 = HashMap::new();
        data2.insert("x".to_string(), serde_json::json!(2));

        bag.set("stage1", data1, 1, false).unwrap();
        bag.set("stage1", data2.clone(), 2, true).unwrap();

        assert_eq!(bag.get("stage1"), Some(data2));
    }

    #[test]
    fn test_output_bag_entry() {
        let bag = OutputBag::new();
        bag.set("stage1", HashMap::new(), 3, true).unwrap();

        let entry = bag.get_entry("stage1").unwrap();
        assert_eq!(entry.attempt, 3);
        assert!(entry.is_final);
    }
}
