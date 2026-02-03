//! Structured task group for managing related async tasks.

use super::{CancellationToken, CleanupRegistry};
use parking_lot::RwLock;
use std::future::Future;
use std::sync::Arc;
use tokio::task::JoinHandle;

/// A group of related tasks with structured cancellation.
///
/// If any task errors, remaining tasks are cancelled.
/// Cleanup is always run on exit.
pub struct StructuredTaskGroup {
    /// The cancellation token for this group.
    cancel_token: Arc<CancellationToken>,
    /// The cleanup registry.
    cleanup_registry: Arc<CleanupRegistry>,
    /// Handles to spawned tasks.
    handles: RwLock<Vec<JoinHandle<Result<(), String>>>>,
    /// The first error encountered.
    first_error: RwLock<Option<String>>,
}

impl StructuredTaskGroup {
    /// Creates a new task group.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cancel_token: Arc::new(CancellationToken::new()),
            cleanup_registry: Arc::new(CleanupRegistry::new()),
            handles: RwLock::new(Vec::new()),
            first_error: RwLock::new(None),
        }
    }

    /// Returns the cancellation token.
    #[must_use]
    pub fn cancel_token(&self) -> &Arc<CancellationToken> {
        &self.cancel_token
    }

    /// Returns the cleanup registry.
    #[must_use]
    pub fn cleanup_registry(&self) -> &Arc<CleanupRegistry> {
        &self.cleanup_registry
    }

    /// Spawns a task in the group.
    pub fn spawn<F, Fut>(&self, name: &str, task: F)
    where
        F: FnOnce(Arc<CancellationToken>) -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        let token = self.cancel_token.clone();
        let handle = tokio::spawn(async move {
            task(token).await
        });

        self.handles.write().push(handle);
    }

    /// Cancels all tasks in the group.
    pub fn cancel_all(&self, reason: &str) {
        self.cancel_token.cancel(reason);
    }

    /// Waits for all tasks to complete.
    ///
    /// If any task fails, remaining tasks are cancelled.
    /// Returns the first error if any occurred.
    pub async fn wait(&self) -> Result<(), String> {
        let handles: Vec<_> = {
            let mut h = self.handles.write();
            std::mem::take(&mut *h)
        };

        for handle in handles {
            match handle.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    // Task returned an error
                    let mut first_error = self.first_error.write();
                    if first_error.is_none() {
                        *first_error = Some(e.clone());
                        self.cancel_token.cancel(&e);
                    }
                }
                Err(join_error) => {
                    // Task panicked or was cancelled
                    let msg = format!("Task join error: {}", join_error);
                    let mut first_error = self.first_error.write();
                    if first_error.is_none() {
                        *first_error = Some(msg.clone());
                        self.cancel_token.cancel(&msg);
                    }
                }
            }
        }

        // Always run cleanup
        self.cleanup_registry.run_all(10.0).await;

        // Return first error
        if let Some(error) = self.first_error.read().clone() {
            Err(error)
        } else {
            Ok(())
        }
    }

    /// Returns the number of pending tasks.
    #[must_use]
    pub fn task_count(&self) -> usize {
        self.handles.read().len()
    }
}

impl Default for StructuredTaskGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for StructuredTaskGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StructuredTaskGroup")
            .field("task_count", &self.task_count())
            .field("cancelled", &self.cancel_token.is_cancelled())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[tokio::test]
    async fn test_task_group_success() {
        let group = StructuredTaskGroup::new();

        group.spawn("task1", |_token| async { Ok(()) });
        group.spawn("task2", |_token| async { Ok(()) });

        let result = group.wait().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_task_group_with_error() {
        let group = StructuredTaskGroup::new();

        group.spawn("success", |_token| async { Ok(()) });
        group.spawn("failure", |_token| async {
            Err("Task failed".to_string())
        });

        let result = group.wait().await;
        assert!(result.is_err());
        assert!(group.cancel_token.is_cancelled());
    }

    #[tokio::test]
    async fn test_task_group_cleanup_always_runs() {
        let group = StructuredTaskGroup::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let counter_clone = counter.clone();
        group.cleanup_registry().register(
            move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            },
            Some("test"),
        );

        group.spawn("task", |_token| async { Ok(()) });

        let _ = group.wait().await;
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_task_group_cleanup_on_error() {
        let group = StructuredTaskGroup::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let counter_clone = counter.clone();
        group.cleanup_registry().register(
            move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            },
            Some("test"),
        );

        group.spawn("failure", |_token| async {
            Err("Failed".to_string())
        });

        let _ = group.wait().await;
        // Cleanup still ran
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_task_respects_cancellation() {
        let group = StructuredTaskGroup::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let counter_clone = counter.clone();
        group.spawn("long_task", move |token| {
            let counter = counter_clone;
            async move {
                for _ in 0..10 {
                    if token.is_cancelled() {
                        return Ok(());
                    }
                    counter.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                Ok(())
            }
        });

        // Give task time to start
        tokio::time::sleep(Duration::from_millis(30)).await;

        group.cancel_all("Manual cancel");

        let _ = group.wait().await;

        // Task should have stopped early
        let count = counter.load(Ordering::SeqCst);
        assert!(count < 10);
    }
}
