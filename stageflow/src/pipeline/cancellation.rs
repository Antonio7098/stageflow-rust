//! Structured cancellation support for pipeline execution.
//!
//! This module provides utilities for proper resource cleanup during
//! pipeline cancellation.
//!
//! Features:
//! - Automatic cleanup of resources on cancellation
//! - Cleanup callbacks with timeout support
//! - Structured concurrency patterns
//! - LIFO ordering for cleanup execution

use parking_lot::Mutex;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

/// Type alias for async cleanup callbacks.
pub type CleanupCallback = Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

/// Registry for cleanup callbacks that run on cancellation.
///
/// Cleanup callbacks are executed in LIFO order (last registered, first executed)
/// to properly unwind resource acquisition.
#[derive(Default)]
pub struct CleanupRegistry {
    callbacks: Mutex<Vec<(String, CleanupCallback)>>,
    completed: Mutex<Vec<String>>,
    failed: Mutex<Vec<(String, String)>>,
}

impl CleanupRegistry {
    /// Creates a new cleanup registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a cleanup callback.
    pub fn register<F, Fut>(&self, name: impl Into<String>, callback: F)
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let name = name.into();
        let boxed: CleanupCallback = Box::new(move || Box::pin(callback()));
        self.callbacks.lock().push((name, boxed));
    }

    /// Returns the number of pending cleanup callbacks.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.callbacks.lock().len()
    }

    /// Runs all cleanup callbacks in LIFO order.
    ///
    /// Returns lists of completed and failed callback names.
    pub async fn run_all(&self, timeout_seconds: f64) -> (Vec<String>, Vec<(String, String)>) {
        let callbacks: Vec<_> = {
            let mut lock = self.callbacks.lock();
            std::mem::take(&mut *lock)
        };

        if callbacks.is_empty() {
            return (Vec::new(), Vec::new());
        }

        // Calculate per-callback timeout
        let per_callback_timeout = Duration::from_secs_f64(
            (timeout_seconds / callbacks.len() as f64).max(0.01)
        );

        let mut completed = Vec::new();
        let mut failed = Vec::new();

        // Execute in reverse order (LIFO)
        for (name, callback) in callbacks.into_iter().rev() {
            let fut = callback();
            match timeout(per_callback_timeout, fut).await {
                Ok(()) => {
                    completed.push(name);
                }
                Err(_) => {
                    failed.push((name, "Timeout".to_string()));
                }
            }
        }

        *self.completed.lock() = completed.clone();
        *self.failed.lock() = failed.clone();

        (completed, failed)
    }

    /// Returns the completed callback names from the last run.
    #[must_use]
    pub fn completed(&self) -> Vec<String> {
        self.completed.lock().clone()
    }

    /// Returns the failed callback names from the last run.
    #[must_use]
    pub fn failed(&self) -> Vec<(String, String)> {
        self.failed.lock().clone()
    }
}

impl std::fmt::Debug for CleanupRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CleanupRegistry")
            .field("pending_count", &self.pending_count())
            .finish()
    }
}

/// Token for coordinating cancellation across tasks.
pub struct CancellationToken {
    cancelled: AtomicBool,
    reason: Mutex<Option<String>>,
    callbacks: Mutex<Vec<Box<dyn FnOnce(String) + Send>>>,
}

impl std::fmt::Debug for CancellationToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CancellationToken")
            .field("cancelled", &self.cancelled.load(Ordering::SeqCst))
            .field("reason", &self.reason.lock())
            .finish()
    }
}

impl CancellationToken {
    /// Creates a new cancellation token.
    #[must_use]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            cancelled: AtomicBool::new(false),
            reason: Mutex::new(None),
            callbacks: Mutex::new(Vec::new()),
        })
    }

    /// Returns true if cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Returns the cancellation reason if cancelled.
    #[must_use]
    pub fn reason(&self) -> Option<String> {
        self.reason.lock().clone()
    }

    /// Requests cancellation with a reason.
    ///
    /// This is idempotent - only the first reason is stored.
    pub fn cancel(&self, reason: impl Into<String>) {
        let reason = reason.into();
        
        // Only set if not already cancelled
        if !self.cancelled.swap(true, Ordering::SeqCst) {
            *self.reason.lock() = Some(reason.clone());
            
            // Run callbacks
            let callbacks: Vec<_> = {
                let mut lock = self.callbacks.lock();
                std::mem::take(&mut *lock)
            };
            
            for callback in callbacks {
                // Suppress errors in callbacks
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    callback(reason.clone());
                })).ok();
            }
        }
    }

    /// Registers a callback to run when cancellation is requested.
    ///
    /// If already cancelled, the callback is invoked immediately.
    pub fn on_cancel<F>(&self, callback: F)
    where
        F: FnOnce(String) + Send + 'static,
    {
        if self.is_cancelled() {
            let reason = self.reason().unwrap_or_default();
            callback(reason);
        } else {
            self.callbacks.lock().push(Box::new(callback));
        }
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self {
            cancelled: AtomicBool::new(false),
            reason: Mutex::new(None),
            callbacks: Mutex::new(Vec::new()),
        }
    }
}

/// Runs a future with cleanup that always executes.
pub async fn run_with_cleanup<T, F, Fut, C, CFut>(
    operation: F,
    cleanup: C,
    cleanup_timeout: Duration,
) -> T
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
    C: FnOnce() -> CFut,
    CFut: Future<Output = ()>,
{
    let result = operation().await;
    
    // Always run cleanup, even on success
    let _ = timeout(cleanup_timeout, cleanup()).await;
    
    result
}

/// Guard that runs cleanup when dropped.
pub struct CleanupGuard {
    cleanup: Option<Box<dyn FnOnce() + Send>>,
}

impl CleanupGuard {
    /// Creates a new cleanup guard.
    pub fn new<F>(cleanup: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Self {
            cleanup: Some(Box::new(cleanup)),
        }
    }

    /// Disarms the guard, preventing cleanup from running.
    pub fn disarm(&mut self) {
        self.cleanup = None;
    }
}

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            cleanup();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[tokio::test]
    async fn test_cleanup_registry_lifo_order() {
        let order = Arc::new(Mutex::new(Vec::new()));
        let registry = CleanupRegistry::new();

        let order1 = order.clone();
        registry.register("first", move || async move {
            order1.lock().push(1);
        });

        let order2 = order.clone();
        registry.register("second", move || async move {
            order2.lock().push(2);
        });

        let order3 = order.clone();
        registry.register("third", move || async move {
            order3.lock().push(3);
        });

        let (completed, failed) = registry.run_all(10.0).await;

        assert_eq!(completed.len(), 3);
        assert!(failed.is_empty());
        
        // Should be LIFO: 3, 2, 1
        let executed_order = order.lock().clone();
        assert_eq!(executed_order, vec![3, 2, 1]);
    }

    #[tokio::test]
    async fn test_cleanup_registry_empty() {
        let registry = CleanupRegistry::new();
        
        let (completed, failed) = registry.run_all(10.0).await;
        
        assert!(completed.is_empty());
        assert!(failed.is_empty());
    }

    #[tokio::test]
    async fn test_cleanup_registry_timeout() {
        let registry = CleanupRegistry::new();

        registry.register("slow", || async {
            tokio::time::sleep(Duration::from_secs(10)).await;
        });

        let (completed, failed) = registry.run_all(0.01).await;

        assert!(completed.is_empty());
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].0, "slow");
    }

    #[test]
    fn test_cancellation_token_initial_state() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
        assert!(token.reason().is_none());
    }

    #[test]
    fn test_cancellation_token_cancel() {
        let token = CancellationToken::new();
        
        token.cancel("User requested");
        
        assert!(token.is_cancelled());
        assert_eq!(token.reason(), Some("User requested".to_string()));
    }

    #[test]
    fn test_cancellation_token_idempotent() {
        let token = CancellationToken::new();
        
        token.cancel("First reason");
        token.cancel("Second reason");
        
        // First reason wins
        assert_eq!(token.reason(), Some("First reason".to_string()));
    }

    #[test]
    fn test_cancellation_token_callback() {
        let token = CancellationToken::new();
        let called = Arc::new(AtomicBool::new(false));
        
        let called_clone = called.clone();
        token.on_cancel(move |_| {
            called_clone.store(true, Ordering::SeqCst);
        });
        
        assert!(!called.load(Ordering::SeqCst));
        
        token.cancel("test");
        
        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_cancellation_token_callback_immediate() {
        let token = CancellationToken::new();
        token.cancel("already cancelled");
        
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();
        
        token.on_cancel(move |_| {
            called_clone.store(true, Ordering::SeqCst);
        });
        
        // Should be called immediately since already cancelled
        assert!(called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_run_with_cleanup() {
        let cleanup_ran = Arc::new(AtomicBool::new(false));
        let cleanup_ran_clone = cleanup_ran.clone();

        let result = run_with_cleanup(
            || async { 42 },
            move || async move {
                cleanup_ran_clone.store(true, Ordering::SeqCst);
            },
            Duration::from_secs(1),
        ).await;

        assert_eq!(result, 42);
        assert!(cleanup_ran.load(Ordering::SeqCst));
    }

    #[test]
    fn test_cleanup_guard() {
        let cleaned = Arc::new(AtomicBool::new(false));
        let cleaned_clone = cleaned.clone();

        {
            let _guard = CleanupGuard::new(move || {
                cleaned_clone.store(true, Ordering::SeqCst);
            });
        }

        assert!(cleaned.load(Ordering::SeqCst));
    }

    #[test]
    fn test_cleanup_guard_disarm() {
        let cleaned = Arc::new(AtomicBool::new(false));
        let cleaned_clone = cleaned.clone();

        {
            let mut guard = CleanupGuard::new(move || {
                cleaned_clone.store(true, Ordering::SeqCst);
            });
            guard.disarm();
        }

        assert!(!cleaned.load(Ordering::SeqCst));
    }
}
