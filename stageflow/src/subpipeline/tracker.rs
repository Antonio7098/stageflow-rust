//! Child run tracker for managing subpipeline references.

use parking_lot::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

/// Information about a child pipeline run.
#[derive(Debug, Clone)]
pub struct ChildRunInfo {
    /// The child's pipeline run ID.
    pub child_run_id: Uuid,
    /// The parent's pipeline run ID.
    pub parent_run_id: Uuid,
    /// The depth level.
    pub depth: u32,
    /// When the child was spawned (ISO 8601).
    pub spawned_at: String,
}

/// Thread-safe tracker for child pipeline runs.
#[derive(Default)]
pub struct ChildRunTracker {
    children: RwLock<HashMap<Uuid, ChildRunInfo>>,
}

impl ChildRunTracker {
    /// Creates a new tracker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a child run.
    pub fn register(&self, info: ChildRunInfo) {
        self.children.write().insert(info.child_run_id, info);
    }

    /// Unregisters a child run.
    pub fn unregister(&self, child_run_id: Uuid) -> Option<ChildRunInfo> {
        self.children.write().remove(&child_run_id)
    }

    /// Gets information about a child run.
    #[must_use]
    pub fn get(&self, child_run_id: Uuid) -> Option<ChildRunInfo> {
        self.children.read().get(&child_run_id).cloned()
    }

    /// Returns all children of a parent.
    #[must_use]
    pub fn children_of(&self, parent_run_id: Uuid) -> Vec<ChildRunInfo> {
        self.children
            .read()
            .values()
            .filter(|info| info.parent_run_id == parent_run_id)
            .cloned()
            .collect()
    }

    /// Returns the number of tracked children.
    #[must_use]
    pub fn len(&self) -> usize {
        self.children.read().len()
    }

    /// Returns true if no children are tracked.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.children.read().is_empty()
    }

    /// Clears all tracked children.
    pub fn clear(&self) {
        self.children.write().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracker_creation() {
        let tracker = ChildRunTracker::new();
        assert!(tracker.is_empty());
    }

    #[test]
    fn test_register_and_get() {
        let tracker = ChildRunTracker::new();
        let child_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();

        let info = ChildRunInfo {
            child_run_id: child_id,
            parent_run_id: parent_id,
            depth: 1,
            spawned_at: crate::utils::iso_timestamp(),
        };

        tracker.register(info.clone());

        assert_eq!(tracker.len(), 1);
        assert!(tracker.get(child_id).is_some());
    }

    #[test]
    fn test_unregister() {
        let tracker = ChildRunTracker::new();
        let child_id = Uuid::new_v4();

        let info = ChildRunInfo {
            child_run_id: child_id,
            parent_run_id: Uuid::new_v4(),
            depth: 1,
            spawned_at: crate::utils::iso_timestamp(),
        };

        tracker.register(info);
        assert!(!tracker.is_empty());

        tracker.unregister(child_id);
        assert!(tracker.is_empty());
    }

    #[test]
    fn test_children_of() {
        let tracker = ChildRunTracker::new();
        let parent_id = Uuid::new_v4();

        for _ in 0..3 {
            let info = ChildRunInfo {
                child_run_id: Uuid::new_v4(),
                parent_run_id: parent_id,
                depth: 1,
                spawned_at: crate::utils::iso_timestamp(),
            };
            tracker.register(info);
        }

        // Add a child with different parent
        let info = ChildRunInfo {
            child_run_id: Uuid::new_v4(),
            parent_run_id: Uuid::new_v4(),
            depth: 1,
            spawned_at: crate::utils::iso_timestamp(),
        };
        tracker.register(info);

        let children = tracker.children_of(parent_id);
        assert_eq!(children.len(), 3);
    }
}
