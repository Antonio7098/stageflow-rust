//! Stage event type for emitting lifecycle and custom events.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An event emitted by a stage during execution.
///
/// Events are used for observability and can be consumed by
/// event sinks for logging, monitoring, or analytics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageEvent {
    /// The event type (e.g., "stage.started", "stage.completed").
    #[serde(rename = "type")]
    pub event_type: String,

    /// When the event occurred (ISO 8601).
    pub timestamp: String,

    /// The event payload data.
    #[serde(default)]
    pub data: HashMap<String, serde_json::Value>,
}

impl StageEvent {
    /// Creates a new stage event.
    #[must_use]
    pub fn new(event_type: impl Into<String>) -> Self {
        Self {
            event_type: event_type.into(),
            timestamp: crate::utils::iso_timestamp(),
            data: HashMap::new(),
        }
    }

    /// Creates a new stage event with data.
    #[must_use]
    pub fn with_data(event_type: impl Into<String>, data: HashMap<String, serde_json::Value>) -> Self {
        Self {
            event_type: event_type.into(),
            timestamp: crate::utils::iso_timestamp(),
            data,
        }
    }

    /// Adds a data field to the event.
    #[must_use]
    pub fn add_data(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.data.insert(key.into(), value);
        self
    }

    /// Converts the event to a dictionary representation.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("type".to_string(), serde_json::json!(self.event_type));
        map.insert("timestamp".to_string(), serde_json::json!(self.timestamp));
        
        if !self.data.is_empty() {
            let data_map: serde_json::Map<String, serde_json::Value> =
                self.data.clone().into_iter().collect();
            map.insert("data".to_string(), serde_json::Value::Object(data_map));
        }
        
        map
    }

    /// Creates a "stage.started" event.
    #[must_use]
    pub fn started(stage_name: &str) -> Self {
        Self::new("stage.started").add_data("stage", serde_json::json!(stage_name))
    }

    /// Creates a "stage.completed" event.
    #[must_use]
    pub fn completed(stage_name: &str, duration_ms: f64) -> Self {
        Self::new("stage.completed")
            .add_data("stage", serde_json::json!(stage_name))
            .add_data("duration_ms", serde_json::json!(duration_ms))
    }

    /// Creates a "stage.failed" event.
    #[must_use]
    pub fn failed(stage_name: &str, error: &str) -> Self {
        Self::new("stage.failed")
            .add_data("stage", serde_json::json!(stage_name))
            .add_data("error", serde_json::json!(error))
    }

    /// Creates a "stage.skipped" event.
    #[must_use]
    pub fn skipped(stage_name: &str, reason: &str) -> Self {
        Self::new("stage.skipped")
            .add_data("stage", serde_json::json!(stage_name))
            .add_data("reason", serde_json::json!(reason))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = StageEvent::new("test.event");
        assert_eq!(event.event_type, "test.event");
        assert!(event.data.is_empty());
    }

    #[test]
    fn test_event_with_data() {
        let mut data = HashMap::new();
        data.insert("key".to_string(), serde_json::json!("value"));

        let event = StageEvent::with_data("test.event", data);
        assert_eq!(event.data.get("key"), Some(&serde_json::json!("value")));
    }

    #[test]
    fn test_event_add_data() {
        let event = StageEvent::new("test.event")
            .add_data("foo", serde_json::json!("bar"))
            .add_data("count", serde_json::json!(42));

        assert_eq!(event.data.len(), 2);
    }

    #[test]
    fn test_event_started() {
        let event = StageEvent::started("my_stage");
        assert_eq!(event.event_type, "stage.started");
        assert_eq!(event.data.get("stage"), Some(&serde_json::json!("my_stage")));
    }

    #[test]
    fn test_event_completed() {
        let event = StageEvent::completed("my_stage", 123.45);
        assert_eq!(event.event_type, "stage.completed");
        assert_eq!(event.data.get("duration_ms"), Some(&serde_json::json!(123.45)));
    }

    #[test]
    fn test_event_serialization() {
        let event = StageEvent::new("test").add_data("x", serde_json::json!(1));
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: StageEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event.event_type, deserialized.event_type);
    }
}
