//! Analytics event types and exporters.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// An analytics event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsEvent {
    /// Event type.
    pub event_type: String,
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
    /// Event data.
    #[serde(default)]
    pub data: HashMap<String, serde_json::Value>,
    /// Pipeline run ID if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline_run_id: Option<Uuid>,
    /// Stage name if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage_name: Option<String>,
    /// Duration in milliseconds if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<f64>,
    /// Additional metadata.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl AnalyticsEvent {
    /// Creates a new analytics event.
    #[must_use]
    pub fn new(event_type: impl Into<String>) -> Self {
        Self {
            event_type: event_type.into(),
            timestamp: Utc::now(),
            data: HashMap::new(),
            pipeline_run_id: None,
            stage_name: None,
            duration_ms: None,
            metadata: HashMap::new(),
        }
    }

    /// Converts to a dictionary.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("event_type".to_string(), serde_json::json!(self.event_type));
        map.insert("timestamp".to_string(), serde_json::json!(self.timestamp.to_rfc3339()));
        map.insert("data".to_string(), serde_json::json!(self.data));

        if let Some(ref id) = self.pipeline_run_id {
            map.insert("pipeline_run_id".to_string(), serde_json::json!(id.to_string()));
        }
        if let Some(ref name) = self.stage_name {
            map.insert("stage_name".to_string(), serde_json::json!(name));
        }
        if let Some(ms) = self.duration_ms {
            map.insert("duration_ms".to_string(), serde_json::json!(ms));
        }
        if !self.metadata.is_empty() {
            map.insert("metadata".to_string(), serde_json::json!(self.metadata));
        }

        map
    }
}

/// JSON file exporter for analytics events.
pub struct JSONFileExporter {
    path: std::path::PathBuf,
    append: bool,
    event_count: std::sync::atomic::AtomicUsize,
}

impl JSONFileExporter {
    /// Creates a new file exporter.
    #[must_use]
    pub fn new(path: impl Into<std::path::PathBuf>, append: bool) -> Self {
        Self {
            path: path.into(),
            append,
            event_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Returns the event count.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.event_count.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// Console exporter for analytics events.
pub struct ConsoleExporter {
    colorize: bool,
    verbose: bool,
    event_count: std::sync::atomic::AtomicUsize,
}

impl ConsoleExporter {
    /// Creates a new console exporter.
    #[must_use]
    pub fn new(colorize: bool, verbose: bool) -> Self {
        Self {
            colorize,
            verbose,
            event_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Returns the event count.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.event_count.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// Buffered exporter with batching.
pub struct BufferedExporter {
    batch_size: usize,
    flush_interval_seconds: f64,
    max_buffer_size: usize,
}

impl BufferedExporter {
    /// Creates a new buffered exporter.
    #[must_use]
    pub fn new(batch_size: usize, flush_interval_seconds: f64, max_buffer_size: usize) -> Self {
        Self {
            batch_size,
            flush_interval_seconds,
            max_buffer_size,
        }
    }
}

/// Analytics sink adapter for EventSink.
pub struct AnalyticsSink {
    exclude_patterns: Vec<String>,
    include_patterns: Vec<String>,
}

impl AnalyticsSink {
    /// Creates a new analytics sink.
    #[must_use]
    pub fn new() -> Self {
        Self {
            exclude_patterns: Vec::new(),
            include_patterns: Vec::new(),
        }
    }
}

impl Default for AnalyticsSink {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analytics_event_creation() {
        let event = AnalyticsEvent::new("test.event");
        assert_eq!(event.event_type, "test.event");
    }

    #[test]
    fn test_analytics_event_to_dict() {
        let mut event = AnalyticsEvent::new("test");
        event.pipeline_run_id = Some(Uuid::new_v4());
        event.duration_ms = Some(100.0);

        let dict = event.to_dict();
        assert!(dict.contains_key("event_type"));
        assert!(dict.contains_key("timestamp"));
        assert!(dict.contains_key("pipeline_run_id"));
        assert!(dict.contains_key("duration_ms"));
    }
}
