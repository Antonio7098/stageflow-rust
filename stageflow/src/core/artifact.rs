//! Stage artifact type for capturing outputs.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An artifact produced by a stage.
///
/// Artifacts represent structured outputs that can be collected
/// and processed independently of the main stage output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageArtifact {
    /// The type of artifact (e.g., "file", "model", "report").
    #[serde(rename = "type")]
    pub artifact_type: String,

    /// A unique identifier for the artifact.
    pub id: String,

    /// The name of the artifact.
    pub name: String,

    /// The artifact data/content.
    pub data: serde_json::Value,

    /// Additional metadata about the artifact.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,

    /// When the artifact was created (ISO 8601).
    pub created_at: String,
}

impl StageArtifact {
    /// Creates a new stage artifact.
    #[must_use]
    pub fn new(
        artifact_type: impl Into<String>,
        id: impl Into<String>,
        name: impl Into<String>,
        data: serde_json::Value,
    ) -> Self {
        Self {
            artifact_type: artifact_type.into(),
            id: id.into(),
            name: name.into(),
            data,
            metadata: HashMap::new(),
            created_at: crate::utils::iso_timestamp(),
        }
    }

    /// Adds metadata to the artifact.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Converts the artifact to a dictionary representation.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("type".to_string(), serde_json::json!(self.artifact_type));
        map.insert("id".to_string(), serde_json::json!(self.id));
        map.insert("name".to_string(), serde_json::json!(self.name));
        map.insert("data".to_string(), self.data.clone());
        map.insert("created_at".to_string(), serde_json::json!(self.created_at));
        
        if !self.metadata.is_empty() {
            let meta_map: serde_json::Map<String, serde_json::Value> =
                self.metadata.clone().into_iter().collect();
            map.insert("metadata".to_string(), serde_json::Value::Object(meta_map));
        }
        
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artifact_creation() {
        let artifact = StageArtifact::new(
            "file",
            "artifact-123",
            "output.txt",
            serde_json::json!({"content": "hello"}),
        );

        assert_eq!(artifact.artifact_type, "file");
        assert_eq!(artifact.id, "artifact-123");
        assert_eq!(artifact.name, "output.txt");
    }

    #[test]
    fn test_artifact_with_metadata() {
        let artifact = StageArtifact::new("model", "m-1", "classifier", serde_json::json!({}))
            .with_metadata("accuracy", serde_json::json!(0.95))
            .with_metadata("version", serde_json::json!("1.0"));

        assert_eq!(artifact.metadata.len(), 2);
        assert_eq!(artifact.metadata.get("accuracy"), Some(&serde_json::json!(0.95)));
    }

    #[test]
    fn test_artifact_serialization() {
        let artifact = StageArtifact::new(
            "test",
            "id-1",
            "test-artifact",
            serde_json::json!({"key": "value"}),
        );

        let json = serde_json::to_string(&artifact).unwrap();
        let deserialized: StageArtifact = serde_json::from_str(&json).unwrap();

        assert_eq!(artifact.artifact_type, deserialized.artifact_type);
        assert_eq!(artifact.id, deserialized.id);
    }
}
