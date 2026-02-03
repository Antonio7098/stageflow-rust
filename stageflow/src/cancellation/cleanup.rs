//! Cleanup registry and utilities.

use parking_lot::RwLock;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tracing::warn;

/// A callback for cleanup, with optional name.
pub struct CleanupCallback {
    /// The callback function.
    callback: Box<dyn Fn() + Send + Sync>,
    /// Optional name for the callback.
    name: Option<String>,
}

/// Registry for cleanup callbacks executed in LIFO order.
#[derive(Default)]
pub struct CleanupRegistry {
    /// Registered callbacks.
    callbacks: RwLock<Vec<CleanupCallback>>,
}

impl CleanupRegistry {
    /// Creates a new cleanup registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a cleanup callback.
    ///
    /// If a name is provided, it's stored with the callback for debugging.
    pub fn register<F>(&self, callback: F, name: Option<&str>)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.callbacks.write().push(CleanupCallback {
            callback: Box::new(callback),
            name: name.map(String::from),
        });
    }

    /// Unregisters a callback by comparing function pointers.
    ///
    /// Returns true if a callback was removed.
    pub fn unregister_by_name(&self, name: &str) -> bool {
        let mut callbacks = self.callbacks.write();
        let initial_len = callbacks.len();
        callbacks.retain(|cb| cb.name.as_deref() != Some(name));
        callbacks.len() < initial_len
    }

    /// Runs all cleanup callbacks in LIFO order.
    ///
    /// Each callback gets a portion of the total timeout.
    /// Failures are collected but don't stop other callbacks from running.
    /// The registry is cleared after completion.
    pub async fn run_all(&self, timeout_secs: f64) -> Vec<(String, String)> {
        let callbacks: Vec<CleanupCallback> = {
            let mut cbs = self.callbacks.write();
            std::mem::take(&mut *cbs)
        };

        if callbacks.is_empty() {
            return Vec::new();
        }

        let per_callback_timeout = (timeout_secs / callbacks.len() as f64).max(0.01);
        let mut failures = Vec::new();

        // Execute in LIFO order (reverse)
        for entry in callbacks.into_iter().rev() {
            let name = entry.name.clone().unwrap_or_else(|| "<unnamed>".to_string());

            let result = tokio::time::timeout(
                Duration::from_secs_f64(per_callback_timeout),
                tokio::task::spawn_blocking(move || {
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        (entry.callback)();
                    }))
                }),
            )
            .await;

            match result {
                Ok(Ok(Ok(()))) => {
                    // Success
                }
                Ok(Ok(Err(panic))) => {
                    let msg = format!("Cleanup callback panicked: {:?}", panic);
                    warn!("{}: {}", name, msg);
                    failures.push((name, msg));
                }
                Ok(Err(join_err)) => {
                    let msg = format!("Cleanup task join error: {}", join_err);
                    warn!("{}: {}", name, msg);
                    failures.push((name, msg));
                }
                Err(_) => {
                    let msg = "Cleanup callback timed out".to_string();
                    warn!("{}: {}", name, msg);
                    failures.push((name, msg));
                }
            }
        }

        failures
    }

    /// Returns the number of pending cleanup callbacks.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.callbacks.read().len()
    }

    /// Clears all registered callbacks without running them.
    pub fn clear(&self) {
        self.callbacks.write().clear();
    }
}

impl std::fmt::Debug for CleanupRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CleanupRegistry")
            .field("pending_count", &self.pending_count())
            .finish()
    }
}

/// Runs a cleanup function in the finally block of an async operation.
pub async fn cleanup_on_cancel<F, Fut, C>(operation: F, cleanup: C)
where
    F: Future<Output = ()>,
    C: FnOnce() + Send,
{
    struct CleanupGuard<C: FnOnce()>(Option<C>);

    impl<C: FnOnce()> Drop for CleanupGuard<C> {
        fn drop(&mut self) {
            if let Some(cleanup) = self.0.take() {
                cleanup();
            }
        }
    }

    let _guard = CleanupGuard(Some(cleanup));
    operation.await;
}

/// Runs a coroutine with cleanup, with timeout on the cleanup.
pub async fn run_with_cleanup<F, Fut, C, CFut>(
    operation: F,
    cleanup: C,
    cleanup_timeout_secs: f64,
) -> Result<(), String>
where
    F: Future<Output = Result<(), String>>,
    C: FnOnce() -> CFut,
    CFut: Future<Output = ()>,
{
    let result = operation.await;

    // Always run cleanup
    let cleanup_result = tokio::time::timeout(
        Duration::from_secs_f64(cleanup_timeout_secs),
        cleanup(),
    )
    .await;

    if cleanup_result.is_err() {
        warn!("Cleanup timed out after {}s", cleanup_timeout_secs);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_registry_creation() {
        let registry = CleanupRegistry::new();
        assert_eq!(registry.pending_count(), 0);
    }

    #[test]
    fn test_registry_register() {
        let registry = CleanupRegistry::new();
        registry.register(|| {}, Some("test"));
        assert_eq!(registry.pending_count(), 1);
    }

    #[tokio::test]
    async fn test_registry_lifo_order() {
        let registry = CleanupRegistry::new();
        let order = Arc::new(RwLock::new(Vec::new()));

        let order1 = order.clone();
        registry.register(move || {
            order1.write().push(1);
        }, Some("first"));

        let order2 = order.clone();
        registry.register(move || {
            order2.write().push(2);
        }, Some("second"));

        let order3 = order.clone();
        registry.register(move || {
            order3.write().push(3);
        }, Some("third"));

        registry.run_all(10.0).await;

        // LIFO: 3, 2, 1
        let result = order.read().clone();
        assert_eq!(result, vec![3, 2, 1]);
    }

    #[tokio::test]
    async fn test_registry_clears_after_run() {
        let registry = CleanupRegistry::new();
        registry.register(|| {}, None);
        assert_eq!(registry.pending_count(), 1);

        registry.run_all(1.0).await;
        assert_eq!(registry.pending_count(), 0);
    }

    #[tokio::test]
    async fn test_registry_continues_on_failure() {
        let registry = CleanupRegistry::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let counter1 = counter.clone();
        registry.register(move || {
            counter1.fetch_add(1, Ordering::SeqCst);
        }, Some("first"));

        registry.register(|| {
            panic!("Intentional");
        }, Some("panics"));

        let counter2 = counter.clone();
        registry.register(move || {
            counter2.fetch_add(1, Ordering::SeqCst);
        }, Some("third"));

        let failures = registry.run_all(10.0).await;

        // All callbacks attempted
        assert_eq!(counter.load(Ordering::SeqCst), 2);
        assert_eq!(failures.len(), 1);
    }

    #[test]
    fn test_unregister_by_name() {
        let registry = CleanupRegistry::new();
        registry.register(|| {}, Some("keep"));
        registry.register(|| {}, Some("remove"));
        registry.register(|| {}, Some("keep2"));

        assert_eq!(registry.pending_count(), 3);

        let removed = registry.unregister_by_name("remove");
        assert!(removed);
        assert_eq!(registry.pending_count(), 2);

        let not_found = registry.unregister_by_name("nonexistent");
        assert!(!not_found);
    }
}
