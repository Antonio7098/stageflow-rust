//! Stage output type with factory methods matching Python semantics.

use super::{StageArtifact, StageEvent, StageStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The output of a stage execution.
///
/// `StageOutput` is immutable once created and provides factory methods
/// for creating outputs with different statuses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageOutput {
    /// The status of the stage execution.
    pub status: StageStatus,

    /// The output data (for successful executions).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<HashMap<String, serde_json::Value>>,

    /// Artifacts produced by the stage.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<StageArtifact>,

    /// Events emitted by the stage.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<StageEvent>,

    /// Additional metadata.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,

    /// Error message (for failed executions).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Skip reason (for skipped executions).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,

    /// Cancel reason (for cancelled executions).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancel_reason: Option<String>,

    /// Whether the error is retryable.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub retryable: bool,
}

impl Default for StageOutput {
    fn default() -> Self {
        Self::ok_empty()
    }
}

impl StageOutput {
    /// Creates a successful output with data.
    #[must_use]
    pub fn ok(data: HashMap<String, serde_json::Value>) -> Self {
        Self {
            status: StageStatus::Ok,
            data: Some(data),
            artifacts: Vec::new(),
            events: Vec::new(),
            metadata: HashMap::new(),
            error: None,
            skip_reason: None,
            cancel_reason: None,
            retryable: false,
        }
    }

    /// Creates a successful output with no data.
    #[must_use]
    pub fn ok_empty() -> Self {
        Self {
            status: StageStatus::Ok,
            data: None,
            artifacts: Vec::new(),
            events: Vec::new(),
            metadata: HashMap::new(),
            error: None,
            skip_reason: None,
            cancel_reason: None,
            retryable: false,
        }
    }

    /// Creates a successful output with a single value.
    #[must_use]
    pub fn ok_value(key: impl Into<String>, value: serde_json::Value) -> Self {
        let mut data = HashMap::new();
        data.insert(key.into(), value);
        Self::ok(data)
    }

    /// Creates a skip output with a reason.
    #[must_use]
    pub fn skip(reason: impl Into<String>) -> Self {
        Self {
            status: StageStatus::Skip,
            data: None,
            artifacts: Vec::new(),
            events: Vec::new(),
            metadata: HashMap::new(),
            error: None,
            skip_reason: Some(reason.into()),
            cancel_reason: None,
            retryable: false,
        }
    }

    /// Creates a cancel output with a reason.
    #[must_use]
    pub fn cancel(reason: impl Into<String>) -> Self {
        Self {
            status: StageStatus::Cancel,
            data: None,
            artifacts: Vec::new(),
            events: Vec::new(),
            metadata: HashMap::new(),
            error: None,
            skip_reason: None,
            cancel_reason: Some(reason.into()),
            retryable: false,
        }
    }

    /// Creates a failure output with an error message.
    #[must_use]
    pub fn fail(error: impl Into<String>) -> Self {
        Self {
            status: StageStatus::Fail,
            data: None,
            artifacts: Vec::new(),
            events: Vec::new(),
            metadata: HashMap::new(),
            error: Some(error.into()),
            skip_reason: None,
            cancel_reason: None,
            retryable: false,
        }
    }

    /// Creates a retryable failure output.
    #[must_use]
    pub fn fail_retryable(error: impl Into<String>) -> Self {
        Self {
            status: StageStatus::Fail,
            data: None,
            artifacts: Vec::new(),
            events: Vec::new(),
            metadata: HashMap::new(),
            error: Some(error.into()),
            skip_reason: None,
            cancel_reason: None,
            retryable: true,
        }
    }

    /// Creates a retry output with a reason.
    #[must_use]
    pub fn retry(reason: impl Into<String>) -> Self {
        Self {
            status: StageStatus::Retry,
            data: None,
            artifacts: Vec::new(),
            events: Vec::new(),
            metadata: HashMap::new(),
            error: Some(reason.into()),
            skip_reason: None,
            cancel_reason: None,
            retryable: true,
        }
    }

    /// Adds artifacts to the output.
    #[must_use]
    pub fn with_artifacts(mut self, artifacts: Vec<StageArtifact>) -> Self {
        self.artifacts = artifacts;
        self
    }

    /// Adds events to the output.
    #[must_use]
    pub fn with_events(mut self, events: Vec<StageEvent>) -> Self {
        self.events = events;
        self
    }

    /// Adds metadata to the output.
    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Adds a single metadata entry.
    #[must_use]
    pub fn add_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Adds data to the output (merges with existing data).
    #[must_use]
    pub fn with_data(mut self, data: HashMap<String, serde_json::Value>) -> Self {
        match &mut self.data {
            Some(existing) => existing.extend(data),
            None => self.data = Some(data),
        }
        self
    }

    /// Returns true if the output indicates success.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    /// Returns true if the output indicates failure.
    #[must_use]
    pub fn is_failure(&self) -> bool {
        self.status.is_failure()
    }

    /// Returns true if the output can be retried.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        self.retryable
    }

    /// Returns the data, or an empty HashMap if none.
    #[must_use]
    pub fn data_or_empty(&self) -> HashMap<String, serde_json::Value> {
        self.data.clone().unwrap_or_default()
    }

    /// Gets a value from the data.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.as_ref().and_then(|d| d.get(key))
    }

    /// Converts the output to a dictionary representation.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("status".to_string(), serde_json::json!(self.status.to_string()));

        if let Some(ref data) = self.data {
            let data_map: serde_json::Map<String, serde_json::Value> =
                data.clone().into_iter().collect();
            map.insert("data".to_string(), serde_json::Value::Object(data_map));
        }

        if !self.artifacts.is_empty() {
            map.insert(
                "artifacts".to_string(),
                serde_json::json!(self.artifacts.iter().map(|a| a.to_dict()).collect::<Vec<_>>()),
            );
        }

        if !self.events.is_empty() {
            map.insert(
                "events".to_string(),
                serde_json::json!(self.events.iter().map(|e| e.to_dict()).collect::<Vec<_>>()),
            );
        }

        if !self.metadata.is_empty() {
            let meta_map: serde_json::Map<String, serde_json::Value> =
                self.metadata.clone().into_iter().collect();
            map.insert("metadata".to_string(), serde_json::Value::Object(meta_map));
        }

        if let Some(ref error) = self.error {
            map.insert("error".to_string(), serde_json::json!(error));
        }

        if let Some(ref reason) = self.skip_reason {
            map.insert("skip_reason".to_string(), serde_json::json!(reason));
        }

        if let Some(ref reason) = self.cancel_reason {
            map.insert("cancel_reason".to_string(), serde_json::json!(reason));
        }

        if self.retryable {
            map.insert("retryable".to_string(), serde_json::json!(true));
        }

        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ok_output() {
        let mut data = HashMap::new();
        data.insert("result".to_string(), serde_json::json!("success"));

        let output = StageOutput::ok(data);
        assert_eq!(output.status, StageStatus::Ok);
        assert!(output.is_success());
        assert!(!output.is_failure());
    }

    #[test]
    fn test_ok_empty() {
        let output = StageOutput::ok_empty();
        assert_eq!(output.status, StageStatus::Ok);
        assert!(output.data.is_none());
    }

    #[test]
    fn test_ok_value() {
        let output = StageOutput::ok_value("key", serde_json::json!("value"));
        assert_eq!(output.get("key"), Some(&serde_json::json!("value")));
    }

    #[test]
    fn test_skip_output() {
        let output = StageOutput::skip("Not needed");
        assert_eq!(output.status, StageStatus::Skip);
        assert_eq!(output.skip_reason, Some("Not needed".to_string()));
        assert!(output.is_success());
    }

    #[test]
    fn test_cancel_output() {
        let output = StageOutput::cancel("User requested");
        assert_eq!(output.status, StageStatus::Cancel);
        assert_eq!(output.cancel_reason, Some("User requested".to_string()));
        assert!(output.is_failure());
    }

    #[test]
    fn test_fail_output() {
        let output = StageOutput::fail("Something went wrong");
        assert_eq!(output.status, StageStatus::Fail);
        assert_eq!(output.error, Some("Something went wrong".to_string()));
        assert!(output.is_failure());
        assert!(!output.is_retryable());
    }

    #[test]
    fn test_fail_retryable() {
        let output = StageOutput::fail_retryable("Temporary error");
        assert!(output.retryable);
        assert!(output.is_retryable());
    }

    #[test]
    fn test_retry_output() {
        let output = StageOutput::retry("Rate limited");
        assert_eq!(output.status, StageStatus::Retry);
        assert!(output.retryable);
    }

    #[test]
    fn test_with_artifacts() {
        let artifact = StageArtifact::new("file", "1", "test", serde_json::json!({}));
        let output = StageOutput::ok_empty().with_artifacts(vec![artifact]);
        assert_eq!(output.artifacts.len(), 1);
    }

    #[test]
    fn test_with_metadata() {
        let output = StageOutput::ok_empty().add_metadata("key", serde_json::json!("value"));
        assert_eq!(output.metadata.get("key"), Some(&serde_json::json!("value")));
    }

    #[test]
    fn test_to_dict() {
        let output = StageOutput::fail("error");
        let dict = output.to_dict();

        assert_eq!(dict.get("status"), Some(&serde_json::json!("fail")));
        assert_eq!(dict.get("error"), Some(&serde_json::json!("error")));
    }

    #[test]
    fn test_serialization() {
        let output = StageOutput::ok_value("x", serde_json::json!(42));
        let json = serde_json::to_string(&output).unwrap();
        let deserialized: StageOutput = serde_json::from_str(&json).unwrap();

        assert_eq!(output.status, deserialized.status);
        assert_eq!(output.get("x"), deserialized.get("x"));
    }
}
