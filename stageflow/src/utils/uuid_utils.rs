//! UUID generation and collision monitoring utilities.

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;
use uuid::Uuid;

/// Generates a new UUID v4.
#[must_use]
pub fn generate_uuid() -> Uuid {
    Uuid::new_v4()
}

/// Generates a new UUID v7 (time-ordered).
#[must_use]
pub fn generate_uuid_v7() -> Uuid {
    Uuid::now_v7()
}

/// Event emitted when a UUID is observed.
#[derive(Debug, Clone)]
pub struct UuidEvent {
    /// The UUID value as a string.
    pub value: String,
    /// Whether this was a collision.
    pub collision: bool,
    /// Optional category for the UUID.
    pub category: Option<String>,
    /// When the UUID was observed.
    pub observed_at: DateTime<Utc>,
    /// Optional skew in milliseconds for UUIDv7.
    pub skew_ms: Option<i64>,
}

/// Entry in the collision monitor window.
#[derive(Debug, Clone)]
struct WindowEntry {
    uuid: String,
    timestamp: f64,
}

/// Monitors for UUID collisions within a sliding time window.
///
/// This is useful for detecting issues with UUID generation in
/// distributed systems.
pub struct UuidCollisionMonitor {
    /// Time-to-live for entries in seconds.
    ttl_seconds: f64,
    /// Maximum number of entries to track.
    max_entries: usize,
    /// The sliding window of observed UUIDs.
    window: RwLock<VecDeque<WindowEntry>>,
    /// Listeners to notify on UUID events.
    listeners: RwLock<Vec<Arc<dyn Fn(UuidEvent) + Send + Sync>>>,
    /// Optional category for emitted events.
    category: Option<String>,
}

impl UuidCollisionMonitor {
    /// Creates a new collision monitor.
    ///
    /// # Arguments
    ///
    /// * `ttl_seconds` - How long to keep entries (minimum 1.0 second)
    /// * `max_entries` - Maximum entries to track (hard cap)
    /// * `category` - Optional category for emitted events
    #[must_use]
    pub fn new(ttl_seconds: f64, max_entries: usize, category: Option<String>) -> Self {
        Self {
            ttl_seconds: ttl_seconds.max(1.0),
            max_entries,
            window: RwLock::new(VecDeque::new()),
            listeners: RwLock::new(Vec::new()),
            category,
        }
    }

    /// Observes a UUID and returns whether it was a collision.
    ///
    /// If the UUID is already in the window, this returns `true` (collision).
    /// Otherwise, adds it to the window and returns `false`.
    pub fn observe(&self, uuid: &str) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);

        let mut window = self.window.write();

        // Trim expired entries
        let cutoff = now - self.ttl_seconds;
        while window.front().map_or(false, |e| e.timestamp < cutoff) {
            window.pop_front();
        }

        // Trim excess entries
        while window.len() >= self.max_entries {
            window.pop_front();
        }

        // Check for collision
        let collision = window.iter().any(|e| e.uuid == uuid);

        // Add new entry
        window.push_back(WindowEntry {
            uuid: uuid.to_string(),
            timestamp: now,
        });

        // Notify listeners
        let event = UuidEvent {
            value: uuid.to_string(),
            collision,
            category: self.category.clone(),
            observed_at: Utc::now(),
            skew_ms: None,
        };

        let listeners = self.listeners.read();
        for listener in listeners.iter() {
            listener(event.clone());
        }

        collision
    }

    /// Registers a listener to be notified on UUID observations.
    pub fn add_listener<F>(&self, listener: F)
    where
        F: Fn(UuidEvent) + Send + Sync + 'static,
    {
        self.listeners.write().push(Arc::new(listener));
    }

    /// Returns the number of entries currently in the window.
    #[must_use]
    pub fn window_size(&self) -> usize {
        self.window.read().len()
    }

    /// Clears all entries from the window.
    pub fn clear(&self) {
        self.window.write().clear();
    }
}

impl Default for UuidCollisionMonitor {
    fn default() -> Self {
        Self::new(60.0, 10000, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_uuid_v4() {
        let id = generate_uuid();
        assert_eq!(id.get_version_num(), 4);
    }

    #[test]
    fn test_generate_uuid_v7() {
        let id = generate_uuid_v7();
        assert_eq!(id.get_version_num(), 7);
    }

    #[test]
    fn test_collision_detection() {
        let monitor = UuidCollisionMonitor::new(10.0, 100, None);

        let uuid = "test-uuid-123";
        assert!(!monitor.observe(uuid)); // First observation
        assert!(monitor.observe(uuid)); // Collision!
    }

    #[test]
    fn test_no_collision_different_uuids() {
        let monitor = UuidCollisionMonitor::new(10.0, 100, None);

        assert!(!monitor.observe("uuid-1"));
        assert!(!monitor.observe("uuid-2"));
        assert!(!monitor.observe("uuid-3"));
    }

    #[test]
    fn test_max_entries_trimming() {
        let monitor = UuidCollisionMonitor::new(1000.0, 3, None);

        monitor.observe("uuid-1");
        monitor.observe("uuid-2");
        monitor.observe("uuid-3");
        assert_eq!(monitor.window_size(), 3);

        monitor.observe("uuid-4"); // Should trim uuid-1
        assert_eq!(monitor.window_size(), 3);

        // uuid-1 should no longer be in window
        assert!(!monitor.observe("uuid-1"));
    }

    #[test]
    fn test_listener_notification() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let monitor = UuidCollisionMonitor::new(10.0, 100, Some("test".to_string()));
        let notified = Arc::new(AtomicBool::new(false));
        let notified_clone = notified.clone();

        monitor.add_listener(move |event| {
            assert_eq!(event.value, "test-uuid");
            assert_eq!(event.category, Some("test".to_string()));
            notified_clone.store(true, Ordering::SeqCst);
        });

        monitor.observe("test-uuid");
        assert!(notified.load(Ordering::SeqCst));
    }
}
