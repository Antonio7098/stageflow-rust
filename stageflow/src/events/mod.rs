//! Event sink system for observability.
//!
//! This module provides the event emission infrastructure used throughout
//! the stageflow framework for logging, monitoring, and analytics.

mod backpressure;
mod sink;

pub use backpressure::{BackpressureAwareEventSink, BackpressureMetrics};
pub use sink::{EventSink, LoggingEventSink, NoOpEventSink};

use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::RwLock as TokioRwLock;

// Global event sink stored in a task-local-like context
static GLOBAL_EVENT_SINK: RwLock<Option<Arc<dyn EventSink>>> = RwLock::new(None);

/// Sets the current global event sink.
pub fn set_event_sink(sink: Arc<dyn EventSink>) {
    *GLOBAL_EVENT_SINK.write() = Some(sink);
}

/// Clears the current global event sink.
pub fn clear_event_sink() {
    *GLOBAL_EVENT_SINK.write() = None;
}

/// Gets the current global event sink.
///
/// Returns a `NoOpEventSink` if no sink is set.
pub fn get_event_sink() -> Arc<dyn EventSink> {
    GLOBAL_EVENT_SINK
        .read()
        .clone()
        .unwrap_or_else(|| Arc::new(NoOpEventSink))
}

/// Tracks pending event sink tasks for cleanup.
static PENDING_TASKS: TokioRwLock<Vec<tokio::task::JoinHandle<()>>> = TokioRwLock::const_new(Vec::new());

/// Registers a pending task for later cleanup.
pub async fn register_pending_task(handle: tokio::task::JoinHandle<()>) {
    PENDING_TASKS.write().await.push(handle);
}

/// Waits for all pending event sink tasks to complete.
pub async fn wait_for_event_sink_tasks() {
    let mut tasks = PENDING_TASKS.write().await;
    if tasks.is_empty() {
        return;
    }

    let handles: Vec<_> = tasks.drain(..).collect();
    drop(tasks); // Release lock before awaiting

    for handle in handles {
        let _ = handle.await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_global_sink_default() {
        clear_event_sink();
        let sink = get_event_sink();
        // Should be a NoOpEventSink (we can't directly check type, but it shouldn't panic)
        sink.try_emit("test", None);
    }

    #[tokio::test]
    async fn test_set_and_get_sink() {
        let sink: Arc<dyn EventSink> = Arc::new(LoggingEventSink::default());
        set_event_sink(sink);

        let retrieved = get_event_sink();
        retrieved.try_emit("test.event", Some(serde_json::json!({"key": "value"})));

        clear_event_sink();
    }
}
