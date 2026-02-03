//! Cancellation token for cooperative cancellation.

use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::warn;

/// A callback type for cancellation notifications.
pub type CancelCallback = Box<dyn Fn() + Send + Sync>;

/// A token for cooperative cancellation.
///
/// Cancellation is idempotent - only the first cancellation reason is kept.
#[derive(Default)]
pub struct CancellationToken {
    /// Whether cancellation has been requested.
    cancelled: AtomicBool,
    /// The reason for cancellation (first one wins).
    reason: RwLock<Option<String>>,
    /// Callbacks to invoke on cancellation.
    callbacks: RwLock<Vec<CancelCallback>>,
}

impl CancellationToken {
    /// Creates a new cancellation token.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests cancellation with a reason.
    ///
    /// This is idempotent - only the first reason is kept.
    /// Callbacks are invoked immediately. Exceptions in callbacks are logged and suppressed.
    pub fn cancel(&self, reason: impl Into<String>) {
        // Only set if not already cancelled (first reason wins)
        if self.cancelled.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
            *self.reason.write() = Some(reason.into());

            // Invoke all callbacks
            let callbacks = self.callbacks.read();
            for callback in callbacks.iter() {
                if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    callback();
                })) {
                    warn!("Cancellation callback panicked: {:?}", e);
                }
            }
        }
    }

    /// Registers a callback to be invoked on cancellation.
    ///
    /// If already cancelled, the callback is invoked immediately.
    pub fn on_cancel<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        if self.is_cancelled() {
            // Already cancelled, invoke immediately
            if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                callback();
            })) {
                warn!("Cancellation callback panicked: {:?}", e);
            }
        } else {
            self.callbacks.write().push(Box::new(callback));
        }
    }

    /// Returns whether cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Returns the cancellation reason, if any.
    #[must_use]
    pub fn reason(&self) -> Option<String> {
        self.reason.read().clone()
    }

    /// Resets the token (for testing).
    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
        *self.reason.write() = None;
        self.callbacks.write().clear();
    }
}

impl std::fmt::Debug for CancellationToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CancellationToken")
            .field("cancelled", &self.is_cancelled())
            .field("reason", &self.reason())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn test_token_default_not_cancelled() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
        assert!(token.reason().is_none());
    }

    #[test]
    fn test_token_cancel() {
        let token = CancellationToken::new();
        token.cancel("User requested");

        assert!(token.is_cancelled());
        assert_eq!(token.reason(), Some("User requested".to_string()));
    }

    #[test]
    fn test_token_cancel_idempotent() {
        let token = CancellationToken::new();
        token.cancel("First reason");
        token.cancel("Second reason");

        // First reason wins
        assert_eq!(token.reason(), Some("First reason".to_string()));
    }

    #[test]
    fn test_on_cancel_before_cancellation() {
        let token = CancellationToken::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        token.on_cancel(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        assert_eq!(counter.load(Ordering::SeqCst), 0);

        token.cancel("test");

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_on_cancel_after_cancellation() {
        let token = CancellationToken::new();
        token.cancel("test");

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        // Should invoke immediately
        token.on_cancel(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_callback_exception_suppressed() {
        let token = CancellationToken::new();

        token.on_cancel(|| {
            panic!("Intentional panic");
        });

        // Should not panic
        token.cancel("test");
        assert!(token.is_cancelled());
    }
}
