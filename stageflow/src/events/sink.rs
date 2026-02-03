//! Event sink trait and implementations.

use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, info, Level};

/// Trait for event sinks that can receive events.
///
/// Event sinks are used throughout stageflow for observability,
/// logging, and analytics.
#[async_trait]
pub trait EventSink: Send + Sync {
    /// Emits an event asynchronously.
    ///
    /// # Arguments
    ///
    /// * `event_type` - The type of event (e.g., "stage.started")
    /// * `data` - Optional event data
    async fn emit(&self, event_type: &str, data: Option<serde_json::Value>);

    /// Tries to emit an event without blocking.
    ///
    /// This method should never raise an exception. Errors are logged
    /// but suppressed.
    fn try_emit(&self, event_type: &str, data: Option<serde_json::Value>);
}

/// A no-op event sink that discards all events.
///
/// Used as the default when no sink is configured.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoOpEventSink;

#[async_trait]
impl EventSink for NoOpEventSink {
    async fn emit(&self, _event_type: &str, _data: Option<serde_json::Value>) {
        // Intentionally empty - discards all events
    }

    fn try_emit(&self, _event_type: &str, _data: Option<serde_json::Value>) {
        // Intentionally empty - discards all events
    }
}

/// An event sink that logs events using the tracing framework.
#[derive(Debug, Clone)]
pub struct LoggingEventSink {
    /// The log level to use.
    level: Level,
}

impl Default for LoggingEventSink {
    fn default() -> Self {
        Self { level: Level::INFO }
    }
}

impl LoggingEventSink {
    /// Creates a new logging event sink with the specified level.
    #[must_use]
    pub fn new(level: Level) -> Self {
        Self { level }
    }

    /// Creates a debug-level logging sink.
    #[must_use]
    pub fn debug() -> Self {
        Self::new(Level::DEBUG)
    }

    /// Creates an info-level logging sink.
    #[must_use]
    pub fn info() -> Self {
        Self::new(Level::INFO)
    }

    fn log_event(&self, event_type: &str, data: &Option<serde_json::Value>) {
        match self.level {
            Level::DEBUG => {
                debug!(
                    event_type = %event_type,
                    event_data = ?data,
                    "Event: {}", event_type
                );
            }
            Level::INFO => {
                info!(
                    event_type = %event_type,
                    event_data = ?data,
                    "Event: {}", event_type
                );
            }
            _ => {
                info!(
                    event_type = %event_type,
                    event_data = ?data,
                    "Event: {}", event_type
                );
            }
        }
    }
}

#[async_trait]
impl EventSink for LoggingEventSink {
    async fn emit(&self, event_type: &str, data: Option<serde_json::Value>) {
        self.log_event(event_type, &data);
    }

    fn try_emit(&self, event_type: &str, data: Option<serde_json::Value>) {
        self.log_event(event_type, &data);
    }
}

/// A collecting event sink for testing purposes.
#[derive(Debug, Default)]
pub struct CollectingEventSink {
    events: parking_lot::RwLock<Vec<(String, Option<serde_json::Value>)>>,
}

impl CollectingEventSink {
    /// Creates a new collecting sink.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns all collected events.
    #[must_use]
    pub fn events(&self) -> Vec<(String, Option<serde_json::Value>)> {
        self.events.read().clone()
    }

    /// Returns the number of collected events.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.read().len()
    }

    /// Returns true if no events have been collected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.read().is_empty()
    }

    /// Clears all collected events.
    pub fn clear(&self) {
        self.events.write().clear();
    }

    /// Returns events matching a type prefix.
    #[must_use]
    pub fn events_of_type(&self, type_prefix: &str) -> Vec<(String, Option<serde_json::Value>)> {
        self.events
            .read()
            .iter()
            .filter(|(t, _)| t.starts_with(type_prefix))
            .cloned()
            .collect()
    }
}

#[async_trait]
impl EventSink for CollectingEventSink {
    async fn emit(&self, event_type: &str, data: Option<serde_json::Value>) {
        self.events.write().push((event_type.to_string(), data));
    }

    fn try_emit(&self, event_type: &str, data: Option<serde_json::Value>) {
        self.events.write().push((event_type.to_string(), data));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_noop_sink() {
        let sink = NoOpEventSink;
        sink.emit("test", None).await;
        sink.try_emit("test", Some(serde_json::json!({"x": 1})));
        // Should not panic
    }

    #[tokio::test]
    async fn test_logging_sink() {
        let sink = LoggingEventSink::default();
        sink.emit("test.event", Some(serde_json::json!({"key": "value"}))).await;
        sink.try_emit("test.event", None);
        // Should not panic
    }

    #[tokio::test]
    async fn test_collecting_sink() {
        let sink = CollectingEventSink::new();
        assert!(sink.is_empty());

        sink.emit("event1", None).await;
        sink.try_emit("event2", Some(serde_json::json!({"data": true})));

        assert_eq!(sink.len(), 2);
        
        let events = sink.events();
        assert_eq!(events[0].0, "event1");
        assert_eq!(events[1].0, "event2");
    }

    #[tokio::test]
    async fn test_collecting_sink_filter() {
        let sink = CollectingEventSink::new();
        sink.emit("stage.started", None).await;
        sink.emit("stage.completed", None).await;
        sink.emit("tool.invoked", None).await;

        let stage_events = sink.events_of_type("stage.");
        assert_eq!(stage_events.len(), 2);

        let tool_events = sink.events_of_type("tool.");
        assert_eq!(tool_events.len(), 1);
    }

    #[tokio::test]
    async fn test_collecting_sink_clear() {
        let sink = CollectingEventSink::new();
        sink.emit("event", None).await;
        assert_eq!(sink.len(), 1);

        sink.clear();
        assert!(sink.is_empty());
    }
}
