//! UUID telemetry helpers for collision detection and instrumentation.

use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use std::collections::{HashSet, VecDeque};
use uuid::Uuid;

/// UUID telemetry data captured by the monitor.
#[derive(Debug, Clone)]
pub struct UuidEvent {
    /// The UUID value observed.
    pub value: Uuid,
    /// Whether this was a collision.
    pub collision: bool,
    /// Logical category for the UUID.
    pub category: String,
    /// When the UUID was observed.
    pub observed_at: DateTime<Utc>,
    /// Clock skew in milliseconds if detected.
    pub skew_ms: Option<f64>,
}

/// Listener callback type for UUID events.
pub type UuidEventListener = Box<dyn Fn(&UuidEvent) + Send + Sync>;

/// Detects if UUIDv7 timestamps deviate significantly from system clock.
#[derive(Debug, Clone)]
pub struct ClockSkewDetector {
    /// Maximum allowed skew in milliseconds.
    pub max_skew_ms: f64,
}

impl Default for ClockSkewDetector {
    fn default() -> Self {
        Self {
            max_skew_ms: 5000.0,
        }
    }
}

impl ClockSkewDetector {
    /// Creates a new clock skew detector.
    #[must_use]
    pub fn new(max_skew_ms: f64) -> Self {
        Self { max_skew_ms }
    }

    /// Check skew for a UUIDv7. Returns skew in ms if significant, else None.
    #[must_use]
    pub fn check(&self, uid: Uuid) -> Option<f64> {
        // UUIDv7 check - version 7 has time-ordered properties
        if uid.get_version_num() != 7 {
            return None;
        }

        // Extract timestamp from UUIDv7 (top 48 bits are milliseconds)
        let bytes = uid.as_bytes();
        let ts_ms = u64::from_be_bytes([
            0, 0, bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
        ]);

        let uuid_time = DateTime::from_timestamp_millis(ts_ms as i64)?;
        let system_time = Utc::now();
        let skew = (system_time - uuid_time).num_milliseconds().unsigned_abs() as f64;

        if skew > self.max_skew_ms {
            Some(skew)
        } else {
            None
        }
    }
}

/// Sliding-window UUID collision detector with optional listeners.
pub struct UuidCollisionMonitor {
    ttl: Duration,
    max_entries: usize,
    category: String,
    entries: RwLock<VecDeque<(DateTime<Utc>, String)>>,
    index: RwLock<HashSet<String>>,
    listeners: RwLock<Vec<UuidEventListener>>,
    skew_detector: Option<ClockSkewDetector>,
}

impl UuidCollisionMonitor {
    /// Creates a new UUID collision monitor.
    #[must_use]
    pub fn new(
        ttl_seconds: f64,
        max_entries: usize,
        category: impl Into<String>,
        check_skew: bool,
    ) -> Self {
        Self {
            ttl: Duration::seconds(ttl_seconds.max(1.0) as i64),
            max_entries: max_entries.max(1),
            category: category.into(),
            entries: RwLock::new(VecDeque::new()),
            index: RwLock::new(HashSet::new()),
            listeners: RwLock::new(Vec::new()),
            skew_detector: if check_skew {
                Some(ClockSkewDetector::default())
            } else {
                None
            },
        }
    }

    /// Creates a monitor with default settings.
    #[must_use]
    pub fn default_with_category(category: impl Into<String>) -> Self {
        Self::new(300.0, 50_000, category, false)
    }

    /// Returns the category.
    #[must_use]
    pub fn category(&self) -> &str {
        &self.category
    }

    /// Register a listener that receives UUID events.
    pub fn add_listener(&self, listener: UuidEventListener) {
        self.listeners.write().push(listener);
    }

    /// Record a UUID and return true if it is a collision within the window.
    pub fn observe(&self, value: Uuid) -> bool {
        let now = Utc::now();
        let key = value.to_string();

        let collision = {
            let index = self.index.read();
            index.contains(&key)
        };

        {
            let mut entries = self.entries.write();
            let mut index = self.index.write();
            entries.push_back((now, key.clone()));
            index.insert(key);
        }

        self.trim(now);

        let skew_ms = self.skew_detector.as_ref().and_then(|d| d.check(value));

        let event = UuidEvent {
            value,
            collision,
            category: self.category.clone(),
            observed_at: now,
            skew_ms,
        };

        let listeners = self.listeners.read();
        for listener in listeners.iter() {
            listener(&event);
        }

        collision
    }

    fn trim(&self, now: DateTime<Utc>) {
        let cutoff = now - self.ttl;
        let mut entries = self.entries.write();
        let mut index = self.index.write();

        while let Some((ts, _)) = entries.front() {
            if *ts < cutoff || entries.len() > self.max_entries {
                if let Some((_, old)) = entries.pop_front() {
                    index.remove(&old);
                }
            } else {
                break;
            }
        }
    }

    /// Returns the number of UUIDs currently tracked.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Returns true if no UUIDs are being tracked.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }
}

/// Generate a new UUIDv4.
#[must_use]
pub fn generate_uuid4() -> Uuid {
    Uuid::new_v4()
}

/// Generate a new UUIDv7 (time-ordered) if available.
#[must_use]
pub fn generate_uuid7() -> Uuid {
    Uuid::now_v7()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_uuid_collision_monitor_no_collision() {
        let monitor = UuidCollisionMonitor::default_with_category("test");
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        assert!(!monitor.observe(uuid1));
        assert!(!monitor.observe(uuid2));
    }

    #[test]
    fn test_uuid_collision_monitor_collision() {
        let monitor = UuidCollisionMonitor::default_with_category("test");
        let uuid = Uuid::new_v4();

        assert!(!monitor.observe(uuid));
        assert!(monitor.observe(uuid)); // Collision
    }

    #[test]
    fn test_uuid_collision_monitor_listener() {
        let monitor = UuidCollisionMonitor::default_with_category("test");
        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();

        monitor.add_listener(Box::new(move |_event| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        }));

        monitor.observe(Uuid::new_v4());
        monitor.observe(Uuid::new_v4());

        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_generate_uuid4() {
        let uuid = generate_uuid4();
        assert_eq!(uuid.get_version_num(), 4);
    }

    #[test]
    fn test_generate_uuid7() {
        let uuid = generate_uuid7();
        assert_eq!(uuid.get_version_num(), 7);
    }

    #[test]
    fn test_clock_skew_detector_v4() {
        let detector = ClockSkewDetector::default();
        let uuid = Uuid::new_v4();
        assert!(detector.check(uuid).is_none()); // Not v7
    }

    #[test]
    fn test_uuid_event_creation() {
        let event = UuidEvent {
            value: Uuid::new_v4(),
            collision: false,
            category: "test".to_string(),
            observed_at: Utc::now(),
            skew_ms: None,
        };
        assert!(!event.collision);
        assert_eq!(event.category, "test");
    }
}
