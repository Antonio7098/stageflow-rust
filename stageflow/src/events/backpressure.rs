//! Backpressure-aware event sink implementation.

use super::{EventSink, LoggingEventSink};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::warn;

/// Metrics for backpressure monitoring.
#[derive(Debug, Default)]
pub struct BackpressureMetrics {
    /// Number of events successfully emitted.
    emitted: AtomicU64,
    /// Number of events dropped.
    dropped: AtomicU64,
    /// Number of times the queue was full.
    queue_full_count: AtomicU64,
    /// Last emit time (as duration since process start).
    last_emit_time: RwLock<Option<Instant>>,
    /// Last drop time (as duration since process start).
    last_drop_time: RwLock<Option<Instant>>,
}

impl BackpressureMetrics {
    /// Records a successful emit.
    pub fn record_emit(&self) {
        self.emitted.fetch_add(1, Ordering::Relaxed);
        *self.last_emit_time.write() = Some(Instant::now());
    }

    /// Records a dropped event.
    pub fn record_drop(&self) {
        self.dropped.fetch_add(1, Ordering::Relaxed);
        self.queue_full_count.fetch_add(1, Ordering::Relaxed);
        *self.last_drop_time.write() = Some(Instant::now());
    }

    /// Returns the number of emitted events.
    #[must_use]
    pub fn emitted(&self) -> u64 {
        self.emitted.load(Ordering::Relaxed)
    }

    /// Returns the number of dropped events.
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Returns the queue full count.
    #[must_use]
    pub fn queue_full_count(&self) -> u64 {
        self.queue_full_count.load(Ordering::Relaxed)
    }

    /// Returns the drop rate as a percentage.
    #[must_use]
    pub fn drop_rate(&self) -> f64 {
        let emitted = self.emitted.load(Ordering::Relaxed);
        let dropped = self.dropped.load(Ordering::Relaxed);
        let total = emitted + dropped;
        if total == 0 {
            0.0
        } else {
            (dropped as f64 / total as f64) * 100.0
        }
    }

    /// Converts metrics to a dictionary.
    #[must_use]
    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "emitted": self.emitted(),
            "dropped": self.dropped(),
            "queue_full_count": self.queue_full_count(),
            "drop_rate_percent": (self.drop_rate() * 100.0).round() / 100.0
        })
    }
}

/// Event message for the internal queue.
struct EventMessage {
    event_type: String,
    data: Option<serde_json::Value>,
}

/// A backpressure-aware event sink that queues events.
///
/// This sink wraps a downstream sink and provides:
/// - Bounded queue to prevent memory exhaustion
/// - Configurable drop behavior when queue is full
/// - Metrics for monitoring backpressure
pub struct BackpressureAwareEventSink {
    /// The downstream sink to emit to.
    downstream: Arc<dyn EventSink>,
    /// Event sender channel.
    tx: mpsc::Sender<EventMessage>,
    /// Event receiver channel (for the worker).
    rx: RwLock<Option<mpsc::Receiver<EventMessage>>>,
    /// Maximum queue size.
    max_queue_size: usize,
    /// Whether the worker is running.
    running: AtomicBool,
    /// Backpressure metrics.
    metrics: Arc<BackpressureMetrics>,
    /// Optional callback when events are dropped.
    on_drop: RwLock<Option<Arc<dyn Fn(&str, &Option<serde_json::Value>) + Send + Sync>>>,
    /// Worker task handle.
    worker_handle: RwLock<Option<tokio::task::JoinHandle<()>>>,
}

impl BackpressureAwareEventSink {
    /// Creates a new backpressure-aware sink.
    #[must_use]
    pub fn new(downstream: Arc<dyn EventSink>, max_queue_size: usize) -> Arc<Self> {
        let (tx, rx) = mpsc::channel(max_queue_size);

        Arc::new(Self {
            downstream,
            tx,
            rx: RwLock::new(Some(rx)),
            max_queue_size,
            running: AtomicBool::new(false),
            metrics: Arc::new(BackpressureMetrics::default()),
            on_drop: RwLock::new(None),
            worker_handle: RwLock::new(None),
        })
    }

    /// Creates a new sink with a logging downstream.
    #[must_use]
    pub fn with_logging(max_queue_size: usize) -> Arc<Self> {
        Self::new(Arc::new(LoggingEventSink::default()), max_queue_size)
    }

    /// Sets the on_drop callback.
    pub fn set_on_drop<F>(&self, callback: F)
    where
        F: Fn(&str, &Option<serde_json::Value>) + Send + Sync + 'static,
    {
        *self.on_drop.write() = Some(Arc::new(callback));
    }

    /// Starts the background worker.
    pub async fn start(self: &Arc<Self>) {
        if self.running.swap(true, Ordering::SeqCst) {
            return; // Already running
        }

        let mut rx = self.rx.write().take();
        if rx.is_none() {
            return;
        }

        let downstream = self.downstream.clone();
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        let handle = tokio::spawn(async move {
            let mut receiver = rx.take().unwrap();
            
            while running_clone.load(Ordering::Relaxed) {
                match tokio::time::timeout(
                    std::time::Duration::from_millis(100),
                    receiver.recv(),
                )
                .await
                {
                    Ok(Some(msg)) => {
                        // Emit to downstream, ignoring errors
                        downstream.emit(&msg.event_type, msg.data).await;
                    }
                    Ok(None) => {
                        // Channel closed
                        break;
                    }
                    Err(_) => {
                        // Timeout, continue loop
                    }
                }
            }
        });

        *self.worker_handle.write() = Some(handle);
    }

    /// Stops the background worker.
    pub async fn stop(&self, drain: bool, timeout_secs: f64) {
        if !self.running.swap(false, Ordering::SeqCst) {
            return; // Not running
        }

        if drain {
            // Wait for queue to drain with timeout
            let deadline = Instant::now() + std::time::Duration::from_secs_f64(timeout_secs);
            while Instant::now() < deadline && !self.tx.is_closed() {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }

        // Cancel worker task
        if let Some(handle) = self.worker_handle.write().take() {
            handle.abort();
            let _ = handle.await;
        }
    }

    /// Returns the current queue size.
    #[must_use]
    pub fn queue_size(&self) -> usize {
        self.max_queue_size - self.tx.capacity()
    }

    /// Returns whether the worker is running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Returns the metrics.
    #[must_use]
    pub fn metrics(&self) -> &BackpressureMetrics {
        &self.metrics
    }
}

#[async_trait]
impl EventSink for BackpressureAwareEventSink {
    async fn emit(&self, event_type: &str, data: Option<serde_json::Value>) {
        let msg = EventMessage {
            event_type: event_type.to_string(),
            data,
        };

        if self.tx.send(msg).await.is_ok() {
            self.metrics.record_emit();
        } else {
            self.metrics.record_drop();
        }
    }

    fn try_emit(&self, event_type: &str, data: Option<serde_json::Value>) {
        let msg = EventMessage {
            event_type: event_type.to_string(),
            data: data.clone(),
        };

        match self.tx.try_send(msg) {
            Ok(()) => {
                self.metrics.record_emit();
            }
            Err(_) => {
                self.metrics.record_drop();

                let queue_size = self.queue_size();
                let dropped_total = self.metrics.dropped();

                warn!(
                    event_type = %event_type,
                    queue_size = %queue_size,
                    dropped_total = %dropped_total,
                    "Event dropped due to backpressure"
                );

                if let Some(ref callback) = *self.on_drop.read() {
                    callback(event_type, &data);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::sink::CollectingEventSink;

    #[test]
    fn test_metrics_default() {
        let metrics = BackpressureMetrics::default();
        assert_eq!(metrics.emitted(), 0);
        assert_eq!(metrics.dropped(), 0);
        assert_eq!(metrics.drop_rate(), 0.0);
    }

    #[test]
    fn test_metrics_recording() {
        let metrics = BackpressureMetrics::default();
        
        metrics.record_emit();
        metrics.record_emit();
        metrics.record_drop();

        assert_eq!(metrics.emitted(), 2);
        assert_eq!(metrics.dropped(), 1);
        assert!((metrics.drop_rate() - 33.333).abs() < 1.0);
    }

    #[test]
    fn test_metrics_to_dict() {
        let metrics = BackpressureMetrics::default();
        metrics.record_emit();
        
        let dict = metrics.to_dict();
        assert_eq!(dict["emitted"], 1);
        assert_eq!(dict["dropped"], 0);
    }

    #[tokio::test]
    async fn test_backpressure_sink_creation() {
        let downstream = Arc::new(CollectingEventSink::new());
        let sink = BackpressureAwareEventSink::new(downstream, 100);
        
        assert!(!sink.is_running());
        assert_eq!(sink.queue_size(), 0);
    }

    #[tokio::test]
    async fn test_backpressure_sink_try_emit() {
        let downstream = Arc::new(CollectingEventSink::new());
        let sink = BackpressureAwareEventSink::new(downstream, 100);
        
        sink.try_emit("test.event", Some(serde_json::json!({"key": "value"})));
        
        assert_eq!(sink.metrics().emitted(), 1);
    }
}
